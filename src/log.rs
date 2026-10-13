//! 呼ばれた記録。Zed のタスクも Stream Deck も画面に何も出さないので、
//! 「呼ばれたのか」「何で止まったのか」を後から見るために %LOCALAPPDATA%\pon\pon.log に 1 行ずつ足す。
//! 書けなくても本処理は止めない。

use std::io::Write;
use std::path::PathBuf;

use crate::zed_env::ZedEnv;

/// これを超えたら古い記録を捨てて書き直す。
const MAX_BYTES: u64 = 256 * 1024;

/// 1 回分の記録。選択テキストは中身を残さず長さだけ（本文が漏れないように）。
pub fn entry(stamp: &str, args: &[String], env: &ZedEnv, outcome: &str) -> String {
    let file = env.file.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "-".into());
    let pos = match (env.row, env.column) {
        (Some(r), Some(c)) => format!("{r}:{c}"),
        _ => "-".into(),
    };
    let outcome = outcome.replace(['\r', '\n'], " / ");
    format!(
        "{stamp}\t{}\tfile={file}\tpos={pos}\tsel={}\t{outcome}",
        args.join(" "),
        env.selected.chars().count()
    )
}

fn path() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("pon");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("pon.log"))
}

pub fn write(line: &str) {
    let Some(path) = path() else { return };
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 一行にまとめ_選択は長さだけ残す() {
        let env = ZedEnv {
            file: Some("D:/n/a.md".into()),
            row: Some(3),
            column: Some(17),
            selected: "ねこ".into(),
        };
        let line = entry("2026-10-13T15:00:00", &["paste".into()], &env, "ok: images/a/neko.png");
        assert_eq!(line, "2026-10-13T15:00:00\tpaste\tfile=D:/n/a.md\tpos=3:17\tsel=2\tok: images/a/neko.png");
    }

    #[test]
    fn 改行は潰し_無い値は_ハイフン() {
        let line = entry("t", &["open".into(), "--today".into()], &ZedEnv::default(), "err: a\nb");
        assert_eq!(line, "t\topen --today\tfile=-\tpos=-\tsel=0\terr: a / b");
    }
}
