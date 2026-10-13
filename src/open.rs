//! 名前からページを開く。無ければ作る。

use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::config;
use crate::pages;
use crate::zed_env::ZedEnv;

pub fn run(
    name: Option<String>,
    today: bool,
    project: Option<PathBuf>,
    env: &ZedEnv,
    ask: impl FnOnce() -> Option<String>,
) -> Result<String> {
    let path = prepare(name, today, project, env, ask)?;
    crate::zed::open(&path)?;
    Ok(path.display().to_string())
}

/// 名前とプロジェクトを決め、規則に当て、無ければ作る。開くのは呼ぶ側。
pub fn prepare(
    name: Option<String>,
    today: bool,
    project: Option<PathBuf>,
    env: &ZedEnv,
    ask: impl FnOnce() -> Option<String>,
) -> Result<PathBuf> {
    let root = decide_root(project, env)?;
    let found = config::find(&root)?.expect("decide_root が pon.toml を確かめている");
    let name = decide_name(name, today, env, ask)?;
    let Some(resolved) = pages::resolve(&found.config, &name)? else {
        bail!("「{name}」に合う規則が pon.toml に無いよ");
    };
    let (path, _created) = pages::ensure(&found.root, &resolved)?;
    Ok(path)
}

fn decide_name(
    name: Option<String>,
    today: bool,
    env: &ZedEnv,
    ask: impl FnOnce() -> Option<String>,
) -> Result<String> {
    if let Some(n) = name {
        return Ok(n);
    }
    if today {
        return Ok(chrono::Local::now().format("%Y-%m-%d").to_string());
    }
    let sel = env.selected.trim();
    if !sel.is_empty() && !sel.contains('\n') {
        return Ok(sel.to_string());
    }
    match ask() {
        Some(n) => Ok(n),
        None => bail!("名前を教えてね"),
    }
}

fn decide_root(project: Option<PathBuf>, env: &ZedEnv) -> Result<PathBuf> {
    let start: PathBuf = match (project, env.file.as_deref()) {
        (Some(p), _) => p,
        (None, Some(f)) => f.to_path_buf(),
        (None, None) => std::env::current_dir()?,
    };
    match config::find(&start)? {
        Some(found) => Ok(found.root),
        None => bail!("{} から上に pon.toml が見つからないよ", start.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            t.path().join("pon.toml"),
            "[[pages]]\nmatch = '^(\\d{4}-\\d{2}-\\d{2})$'\npath = 'diary/$1.md'\ntemplate = '# $1\\n'\n\n[[pages]]\nmatch = '^([a-z]+)$'\npath = '$1.md'\n",
        )
        .unwrap();
        t
    }

    #[test]
    fn 名前の順は_引数_today_選択_聞く() {
        let t = project();
        let env_sel = ZedEnv { file: Some(t.path().join("x.md")), selected: "neko".into(), ..Default::default() };
        let env_none = ZedEnv::default();

        let p = decide_name(Some("inu".into()), true, &env_sel, || Some("asked".into())).unwrap();
        assert_eq!(p, "inu");
        let p = decide_name(None, true, &env_sel, || Some("asked".into())).unwrap();
        assert_eq!(p.len(), 10); // YYYY-MM-DD
        let p = decide_name(None, false, &env_sel, || Some("asked".into())).unwrap();
        assert_eq!(p, "neko");
        let p = decide_name(None, false, &env_none, || Some("asked".into())).unwrap();
        assert_eq!(p, "asked");
        assert!(decide_name(None, false, &env_none, || None).is_err());
    }

    #[test]
    fn プロジェクトは_project_file_cwd_の順() {
        let t = project();
        let env = ZedEnv { file: Some(t.path().join("a").join("b.md")), ..Default::default() };
        assert_eq!(decide_root(Some(t.path().to_path_buf()), &ZedEnv::default()).unwrap(), t.path());
        assert_eq!(decide_root(None, &env).unwrap(), t.path());
        let none = tempfile::tempdir().unwrap();
        assert!(decide_root(Some(none.path().to_path_buf()), &ZedEnv::default()).is_err());
    }

    #[test]
    fn 作って開くパスを返す() {
        let t = project();
        let path = prepare(Some("neko".into()), false, Some(t.path().to_path_buf()), &ZedEnv::default(), || None).unwrap();
        assert_eq!(path, t.path().join("neko.md"));
        assert!(path.exists());
    }

    #[test]
    fn 規則に当たらなければ_err() {
        let t = project();
        let e = prepare(Some("ねこ".into()), false, Some(t.path().to_path_buf()), &ZedEnv::default(), || None).unwrap_err();
        assert!(e.to_string().contains("規則"));
    }
}
