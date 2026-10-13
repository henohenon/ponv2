//! 名前を規則に当てて、ページのパスと雛形を決める。

use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;

use crate::config::Config;

#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub rel_path: String,
    pub template: String,
}

pub fn resolve(config: &Config, name: &str) -> Result<Option<Resolved>> {
    for page in &config.pages {
        let re = Regex::new(&page.pattern)
            .with_context(|| format!("pon.toml ノ match ガ フセイ デス：{}", page.pattern))?;
        let Some(caps) = re.captures(name) else { continue };
        let mut rel_path = String::new();
        caps.expand(&page.path, &mut rel_path);
        let mut template = String::new();
        caps.expand(&page.template, &mut template);
        return Ok(Some(Resolved { rel_path, template }));
    }
    Ok(None)
}

/// ルートからの相対パスを絶対にする。ルートの外に出るものは拒む。
pub fn target_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = Path::new(rel);
    if rel.is_absolute() || rel.has_root() {
        bail!("パスハ ソウタイデ カイテクダサイ：{}", rel.display());
    }
    let mut depth = 0usize;
    for c in rel.components() {
        match c {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                if depth == 0 {
                    bail!("プロジェクトノ ソトニ デル パス デス：{}", rel.display());
                }
                depth -= 1;
            }
            Component::Prefix(_) | Component::RootDir => bail!("パスハ ソウタイデ カイテクダサイ：{}", rel.display()),
        }
    }
    Ok(root.join(rel))
}

/// 無ければ途中のフォルダごと作って雛形を書く。あれば何もしない。
pub fn ensure(root: &Path, resolved: &Resolved) -> Result<(PathBuf, bool)> {
    let path = target_path(root, &resolved.rel_path)?;
    if path.exists() {
        return Ok((path, false));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &resolved.template).with_context(|| format!("{} ヲ ツクレマセン。", path.display()))?;
    Ok((path, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config::parse(
            r#"
[[pages]]
match = '^(\d{4})-(\d{2})-(\d{2})$'
path = "!/diary/$1-$2-$3/!.md"
template = "---\ntitle: $1-$2-$3\n---\n"

[[pages]]
match = '^([a-z0-9-]+)$'
path = "!/$1/!.md"
"#,
        )
        .unwrap()
    }

    #[test]
    fn 上から順に最初の一致() {
        let r = resolve(&cfg(), "2026-10-13").unwrap().unwrap();
        assert_eq!(r.rel_path, "!/diary/2026-10-13/!.md");
        assert_eq!(r.template, "---\ntitle: 2026-10-13\n---\n");
        let r = resolve(&cfg(), "neko").unwrap().unwrap();
        assert_eq!(r, Resolved { rel_path: "!/neko/!.md".into(), template: "".into() });
    }

    #[test]
    fn どれにも当たらなければ_none() {
        assert_eq!(resolve(&cfg(), "ねこ").unwrap(), None);
        assert_eq!(resolve(&cfg(), "").unwrap(), None);
    }

    #[test]
    fn 壊れた正規表現は_err() {
        let c = Config::parse("[[pages]]\nmatch = '('\npath = 'x.md'\n").unwrap();
        assert!(resolve(&c, "a").is_err());
    }

    #[test]
    fn 親を指すパスは拒む() {
        assert!(target_path(Path::new("D:/p"), "../x.md").is_err());
        assert!(target_path(Path::new("D:/p"), "a/../../x.md").is_err());
        assert!(target_path(Path::new("D:/p"), "D:/other/x.md").is_err());
        assert_eq!(target_path(Path::new("D:/p"), "!/neko/!.md").unwrap(), PathBuf::from("D:/p/!/neko/!.md"));
    }

    #[test]
    fn 無ければ雛形で作り_あれば触らない() {
        let t = tempfile::tempdir().unwrap();
        let r = Resolved { rel_path: "!/neko/!.md".into(), template: "---\ntitle: neko\n---\n".into() };

        let (path, created) = ensure(t.path(), &r).unwrap();
        assert!(created);
        assert_eq!(fs::read_to_string(&path).unwrap(), "---\ntitle: neko\n---\n");

        fs::write(&path, "edited").unwrap();
        let (_, created) = ensure(t.path(), &r).unwrap();
        assert!(!created);
        assert_eq!(fs::read_to_string(&path).unwrap(), "edited");
    }
}
