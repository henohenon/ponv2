//! Zed がタスクに ZED_FILE を渡さないとき（ファイル単体で開いているとき）の代わり。
//!
//! Zed は前面のタブ（items.active）、各タブのファイル（editors.path）、カーソルと選択（editor_selections、
//! バイト位置）を状態 DB（%LOCALAPPDATA%\Zed\db\0-stable\db.sqlite、WAL）に書いている。書き込みは
//! 変更から 0.2 秒後なので、キーを押す頃にはふつう反映されている。これを読み取り専用で読み、
//! 前面の Zed の窓タイトル（「プロジェクト — ファイル」）で、どのワークスペースかを決める。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

use crate::zed_env::ZedEnv;

/// あるワークスペースの、前面のタブ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// ワークスペースが開いているフォルダやファイル（DB の paths を改行で分けた物）。
    pub roots: Vec<PathBuf>,
    pub timestamp: String,
    pub file: PathBuf,
    /// カーソル（start == end）か選択の範囲。Zed のバッファ（改行は LF）でのバイト位置。
    pub selection: Option<(usize, usize)>,
}

/// 「notes — 2026-10-13.md」→ (Some("notes"), "2026-10-13.md")。区切りが無ければファイル名だけ。
pub fn parse_title(title: &str) -> Option<(Option<&str>, &str)> {
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    let parts: Vec<&str> = title.split(" — ").collect();
    match parts.as_slice() {
        [only] => Some((None, only)),
        [first, .., last] => Some((Some(first), last)),
        [] => None,
    }
}

/// LF のテキストのバイト位置を、1 始まりの行と列（行内のバイト位置）にする。末尾より後ろは末尾に寄せる。
pub fn row_col(text: &str, offset: usize) -> (usize, usize) {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let row = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (row, offset - line_start + 1)
}

fn file_name(p: &Path) -> Option<&str> {
    p.file_name().and_then(|s| s.to_str())
}

/// 窓タイトルに合う候補を選ぶ。ファイル名は必須、プロジェクト名が合う物を優先、残りは新しいワークスペース。
pub fn pick<'a>(cs: &'a [Candidate], title: Option<(Option<&str>, &str)>) -> Option<&'a Candidate> {
    let Some((project, name)) = title else {
        return cs.iter().max_by(|a, b| a.timestamp.cmp(&b.timestamp));
    };
    let by_name: Vec<&Candidate> = cs.iter().filter(|c| file_name(&c.file) == Some(name)).collect();
    let by_project: Vec<&Candidate> = match project {
        Some(p) => by_name.iter().copied().filter(|c| c.roots.iter().any(|r| file_name(r) == Some(p))).collect(),
        None => Vec::new(),
    };
    let pool = if by_project.is_empty() { by_name } else { by_project };
    pool.into_iter().max_by(|a, b| a.timestamp.cmp(&b.timestamp))
}

/// 候補と、そのファイルの中身（ディスク上、CRLF のこともある）から、Zed のタスク相当の値を作る。
pub fn to_env(c: &Candidate, content: &str) -> ZedEnv {
    let text = content.replace("\r\n", "\n");
    let (row, column, selected) = match c.selection {
        Some((start, end)) => {
            let (row, col) = row_col(&text, start);
            let (s, e) = (start.min(text.len()), end.min(text.len()));
            let selected = text.get(s..e).unwrap_or("").to_string();
            (Some(row), Some(col), selected)
        }
        None => (None, None, String::new()),
    };
    ZedEnv { file: Some(c.file.clone()), row, column, selected }
}

fn text_or_blob(v: rusqlite::types::ValueRef<'_>) -> String {
    match v {
        rusqlite::types::ValueRef::Text(b) | rusqlite::types::ValueRef::Blob(b) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    }
}

/// 状態 DB から、ワークスペースごとの前面のタブ（エディタ）を読む。
pub fn read_candidates(db: &Path) -> Result<Vec<Candidate>> {
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .with_context(|| format!("{} ヲ 開ケマセン。", db.display()))?;
    conn.busy_timeout(std::time::Duration::from_millis(200))?;
    let mut stmt = conn.prepare(
        "SELECT w.paths, w.timestamp, e.path,
                (SELECT s.start FROM editor_selections s
                  WHERE s.editor_id = e.item_id AND s.workspace_id = e.workspace_id ORDER BY s.item_id LIMIT 1),
                (SELECT s.end FROM editor_selections s
                  WHERE s.editor_id = e.item_id AND s.workspace_id = e.workspace_id ORDER BY s.item_id LIMIT 1)
           FROM workspaces w
           JOIN panes p   ON p.workspace_id = w.workspace_id AND p.active = 1
           JOIN items i   ON i.workspace_id = w.workspace_id AND i.pane_id = p.pane_id AND i.active = 1
           JOIN editors e ON e.workspace_id = w.workspace_id AND e.item_id = i.item_id",
    )?;
    let rows = stmt.query_map([], |r| {
        let roots = text_or_blob(r.get_ref(0)?).lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect();
        let timestamp = text_or_blob(r.get_ref(1)?);
        let file = PathBuf::from(text_or_blob(r.get_ref(2)?));
        let start: Option<i64> = r.get(3)?;
        let end: Option<i64> = r.get(4)?;
        let selection = match (start, end) {
            (Some(s), Some(e)) if s >= 0 && e >= 0 => Some((s.min(e) as usize, s.max(e) as usize)),
            _ => None,
        };
        Ok(Candidate { roots, timestamp, file, selection })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// 前面の Zed の窓と状態 DB から、今のファイル・カーソル・選択を組み立てる。取れなければ None。
pub fn current() -> Option<ZedEnv> {
    let title = crate::win::foreground_zed_title()?;
    let db = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Zed").join("db").join("0-stable").join("db.sqlite");
    let cs = read_candidates(&db).ok()?;
    let c = pick(&cs, parse_title(&title))?;
    let content = std::fs::read_to_string(&c.file).ok()?;
    Some(to_env(c, &content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 窓タイトルからプロジェクトとファイルを取る() {
        assert_eq!(parse_title("notes — 2026-10-13.md"), Some((Some("notes"), "2026-10-13.md")));
        assert_eq!(parse_title("2026-10-13.md"), Some((None, "2026-10-13.md")));
        assert_eq!(parse_title(""), None);
    }

    #[test]
    fn バイト位置を行と列にする() {
        assert_eq!(row_col("ab\ncd", 0), (1, 1));
        assert_eq!(row_col("ab\ncd", 4), (2, 2));
        // 「ねこ」は 6 バイト。直後は 7 列目。
        assert_eq!(row_col("x\nねこ\n", 8), (2, 7));
        // 末尾より後ろ（保存で末尾が削られた）は末尾に寄せる。
        assert_eq!(row_col("ab\n", 99), (2, 1));
    }

    fn cand(roots: &[&str], ts: &str, file: &str) -> Candidate {
        Candidate {
            roots: roots.iter().map(PathBuf::from).collect(),
            timestamp: ts.into(),
            file: file.into(),
            selection: Some((0, 0)),
        }
    }

    #[test]
    fn タイトルのファイル名とプロジェクト名で選ぶ() {
        let cs = vec![
            cand(&["D:/a/blog"], "2026-10-13 06:30:00", "D:/a/blog/!/foo/!.md"),
            cand(&["D:/a/notes"], "2026-10-13 06:20:00", "D:/a/notes/2026-10-13.md"),
            cand(&["D:/a/other"], "2026-10-13 06:40:00", "D:/a/other/!.md"),
        ];
        // ファイル名「!.md」は 2 つあるが、プロジェクト名で blog に決まる。
        assert_eq!(pick(&cs, Some((Some("blog"), "!.md"))).unwrap().file, PathBuf::from("D:/a/blog/!/foo/!.md"));
        // ファイル単体のワークスペースはタイトルにプロジェクト名が無い。ファイル名だけで決まる。
        assert_eq!(pick(&cs, Some((None, "2026-10-13.md"))).unwrap().file, PathBuf::from("D:/a/notes/2026-10-13.md"));
        // ファイル名が合う物が無ければ選ばない（取り違えるより止まる方がよい）。
        assert_eq!(pick(&cs, Some((None, "nothing.md"))), None);
    }

    #[test]
    fn 同じファイル名なら新しいワークスペースを選ぶ() {
        let cs = vec![
            cand(&["D:/a/notes/2026-10-13.md"], "2026-10-13 06:00:00", "D:/a/notes/2026-10-13.md"),
            cand(&["D:/a/notes"], "2026-10-13 06:30:00", "D:/a/notes/2026-10-13.md"),
        ];
        assert_eq!(pick(&cs, Some((None, "2026-10-13.md"))).unwrap().roots, vec![PathBuf::from("D:/a/notes")]);
    }

    #[test]
    fn 候補とファイルの中身から_zed_env_を組み立てる() {
        let c = Candidate { selection: Some((3, 7)), ..cand(&["D:/n"], "t", "D:/n/a.md") };
        // ファイルは CRLF でも、Zed のバッファは LF なので位置は LF で数える。
        let env = to_env(&c, "ab\r\nneko\r\n");
        assert_eq!(env.file, Some(PathBuf::from("D:/n/a.md")));
        assert_eq!((env.row, env.column), (Some(2), Some(1)));
        assert_eq!(env.selected, "neko");
    }

    /// 手で回す確認用：`cargo test 実機 -- --ignored --nocapture`。Zed が動いている間に読めるか。
    #[test]
    #[ignore]
    fn 実機の状態_db_を読める() {
        let db = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Zed/db/0-stable/db.sqlite");
        let t0 = std::time::Instant::now();
        let cs = read_candidates(&db).unwrap();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        for c in &cs {
            println!("{} {:?} {} {:?}", c.timestamp, c.roots, c.file.display(), c.selection);
        }
        println!("{} workspaces, {ms:.2} ms", cs.len());
        assert!(!cs.is_empty());
    }

    #[test]
    fn db_から各ワークスペースの前面タブを読む() {
        let t = tempfile::tempdir().unwrap();
        let db = t.path().join("db.sqlite");
        let c = Connection::open(&db).unwrap();
        c.execute_batch(
            "CREATE TABLE workspaces (workspace_id INTEGER, paths TEXT, timestamp TEXT);
             CREATE TABLE panes (pane_id INTEGER, workspace_id INTEGER, active INTEGER);
             CREATE TABLE items (item_id INTEGER, workspace_id INTEGER, pane_id INTEGER, kind TEXT, active INTEGER);
             CREATE TABLE editors (item_id INTEGER, workspace_id INTEGER, path BLOB);
             CREATE TABLE editor_selections (item_id INTEGER PRIMARY KEY, editor_id INTEGER, workspace_id INTEGER, start INTEGER, end INTEGER);
             INSERT INTO workspaces VALUES (1, 'D:\\n', '2026-10-13 06:00:00');
             INSERT INTO panes VALUES (10, 1, 1);
             INSERT INTO items VALUES (100, 1, 10, 'Editor', 0);
             INSERT INTO items VALUES (101, 1, 10, 'Editor', 1);
             INSERT INTO editors VALUES (100, 1, CAST('D:\\n\\old.md' AS BLOB));
             INSERT INTO editors VALUES (101, 1, CAST('D:\\n\\今日.md' AS BLOB));
             INSERT INTO editor_selections (editor_id, workspace_id, start, end) VALUES (101, 1, 3, 7);",
        )
        .unwrap();
        drop(c);

        let cs = read_candidates(&db).unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].file, PathBuf::from("D:\\n\\今日.md"));
        assert_eq!(cs[0].roots, vec![PathBuf::from("D:\\n")]);
        assert_eq!(cs[0].selection, Some((3, 7)));
    }
}
