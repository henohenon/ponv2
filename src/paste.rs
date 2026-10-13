//! クリップボードの画像を置き場に保存し、本文にリンクを入れる。

use std::fs;

use anyhow::{bail, Result};

use crate::config::{self, Config};
use crate::images;
use crate::markdown;
use crate::names::{is_valid_stem, next_free};
use crate::zed_env::ZedEnv;

pub fn run(env: &ZedEnv) -> Result<String> {
    let md = env.md_file()?;
    let found = config::find(md)?;
    let png = crate::clipboard::read_png()?;
    run_with(env, found.as_ref().map(|f| &f.config), &png)
}

pub fn run_with(env: &ZedEnv, cfg: Option<&Config>, png: &[u8]) -> Result<String> {
    if png.is_empty() {
        bail!("クリップボードに画像が無い");
    }
    let md = env.md_file()?;
    let (row, column) = match (env.row, env.column) {
        (Some(r), Some(c)) => (r, c),
        _ => bail!("ZED_ROW / ZED_COLUMN が無い"),
    };
    let content = fs::read_to_string(md)?;

    let dest_dir = images::dir(cfg, md);
    let selected = env.selected.trim();
    let (name, replace) = if is_valid_stem(selected) {
        (format!("{selected}.png"), env.selected.as_str())
    } else {
        (next_free(&dest_dir, "image", ".png"), "")
    };
    let dest = dest_dir.join(&name);
    if dest.exists() {
        bail!("{} は既にある", dest.display());
    }
    let link = images::link_for(md, &dest)?;
    let out = markdown::insert_at(&content, row, column, replace, &format!("![]({link})"))?;

    // ここから先でだけディスクに触る。
    fs::create_dir_all(&dest_dir)?;
    fs::write(&dest, png)?;
    fs::write(md, out)?;
    Ok(link)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

    fn env(file: PathBuf, row: usize, column: usize, selected: &str) -> ZedEnv {
        ZedEnv { file: Some(file), row: Some(row), column: Some(column), selected: selected.into() }
    }

    #[test]
    fn 選択が名前になり_選択がリンクに置き換わる() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "a\nneko\n").unwrap();

        let msg = run_with(&env(md.clone(), 2, 1, "neko"), None, PNG).unwrap();

        assert_eq!(msg, "neko.png");
        assert_eq!(fs::read(t.path().join("neko.png")).unwrap(), PNG);
        assert_eq!(fs::read_to_string(&md).unwrap(), "a\n![](neko.png)\n");
    }

    #[test]
    fn 選択が無ければ連番でカーソル位置に入れる() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "a\n\nb\n").unwrap();
        fs::write(t.path().join("image.png"), b"").unwrap();

        run_with(&env(md.clone(), 2, 1, ""), None, PNG).unwrap();

        assert!(t.path().join("image_1.png").exists());
        assert_eq!(fs::read_to_string(&md).unwrap(), "a\n![](image_1.png)\nb\n");
    }

    #[test]
    fn 選択が名前に使えなければ連番にして選択は残す() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "ねこの写真\n").unwrap();

        run_with(&env(md.clone(), 1, 1, "ねこの写真"), None, PNG).unwrap();

        assert_eq!(fs::read_to_string(&md).unwrap(), "![](image.png)ねこの写真\n");
    }

    #[test]
    fn 設定の置き場に保存して相対リンクを書く() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("2026-10-13.md");
        fs::write(&md, "\n").unwrap();
        let cfg = Config::parse("[images]\ndir = 'images/{stem}'\n").unwrap();

        run_with(&env(md.clone(), 1, 1, "neko"), Some(&cfg), PNG).unwrap();

        assert!(t.path().join("images/2026-10-13/neko.png").exists());
        assert_eq!(fs::read_to_string(&md).unwrap(), "![](images/2026-10-13/neko.png)\n");
    }

    #[test]
    fn 同名があれば止まり_何も変えない() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "neko\n").unwrap();
        fs::write(t.path().join("neko.png"), b"old").unwrap();

        assert!(run_with(&env(md.clone(), 1, 1, "neko"), None, PNG).is_err());

        assert_eq!(fs::read(t.path().join("neko.png")).unwrap(), b"old");
        assert_eq!(fs::read_to_string(&md).unwrap(), "neko\n");
    }

    #[test]
    fn 画像が空なら止まる() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "\n").unwrap();
        let e = run_with(&env(md, 1, 1, ""), None, b"").unwrap_err();
        assert!(e.to_string().contains("画像が無い"));
    }
}
