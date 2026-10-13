//! プロジェクトの設定 pon.toml。起点のフォルダから上へ辿って最初の 1 つを使う。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub images: Images,
    #[serde(default)]
    pub pages: Vec<Page>,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Images {
    /// 画像の置き場。md のフォルダからの相対。`{stem}` は md のファイル名（拡張子なし）。
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(rename = "match")]
    pub pattern: String,
    /// プロジェクトのルートからの相対。`$1` などの捕捉が使える。
    pub path: String,
    #[serde(default)]
    pub template: String,
}

pub struct Found {
    pub root: PathBuf,
    pub config: Config,
}

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        toml::from_str(text).context("pon.toml ヲ ヨメマセン。")
    }
}

/// `start`（ファイルならその親）から上へ辿り、最初の pon.toml を返す。
pub fn find(start: &Path) -> Result<Option<Found>> {
    let mut dir = if start.is_file() { start.parent() } else { Some(start) };
    while let Some(d) = dir {
        let file = d.join("pon.toml");
        if file.is_file() {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("{} ヲ ヨメマセン。", file.display()))?;
            let config = Config::parse(&text).with_context(|| format!("{}", file.display()))?;
            return Ok(Some(Found { root: d.to_path_buf(), config }));
        }
        dir = d.parent();
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn toml_を読む() {
        let c = Config::parse(
            r#"
[images]
dir = "images/{stem}"

[[pages]]
match = '^(\d{4}-\d{2}-\d{2})$'
path = "!/diary/$1/!.md"
template = "---\ntitle: $1\n---\n"

[[pages]]
match = '^(.+)$'
path = "$1.md"
"#,
        )
        .unwrap();
        assert_eq!(c.images.dir.as_deref(), Some("images/{stem}"));
        assert_eq!(c.pages.len(), 2);
        assert_eq!(c.pages[0].pattern, r"^(\d{4}-\d{2}-\d{2})$");
        assert_eq!(c.pages[1].template, "");
    }

    #[test]
    fn 空でも読める_知らないキーは_err() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert!(Config::parse("[images]\nfolder = 'x'").is_err());
    }

    #[test]
    fn 上へ辿って最初の_pon_toml_を使う() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("proj");
        let deep = root.join("a").join("b");
        fs::create_dir_all(&deep).unwrap();
        fs::write(root.join("pon.toml"), "[images]\ndir = 'img'\n").unwrap();

        let f = find(&deep).unwrap().unwrap();
        assert_eq!(f.root, root);
        assert_eq!(f.config.images.dir.as_deref(), Some("img"));
    }

    #[test]
    fn 無ければ_none() {
        let t = tempfile::tempdir().unwrap();
        assert!(find(t.path()).unwrap().is_none());
    }
}
