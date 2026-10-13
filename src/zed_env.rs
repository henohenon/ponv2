//! Zed のタスクが環境変数で渡してくる編集中の状態。
//! ZED_ROW / ZED_COLUMN は 1 始まり。ZED_COLUMN は行内の UTF-8 バイト位置。

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ZedEnv {
    pub file: Option<PathBuf>,
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub selected: String,
}

impl ZedEnv {
    pub fn from_env() -> Self {
        Self::read(|k| std::env::var(k).ok())
    }

    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            file: get("ZED_FILE").filter(|s| !s.is_empty()).map(PathBuf::from),
            row: get("ZED_ROW").and_then(|s| s.parse().ok()),
            column: get("ZED_COLUMN").and_then(|s| s.parse().ok()),
            selected: get("ZED_SELECTED_TEXT").unwrap_or_default(),
        }
    }

    /// 編集中の Markdown。Zed のタスクから呼ばれていなければ Err。
    pub fn md_file(&self) -> Result<&Path> {
        let Some(file) = self.file.as_deref() else {
            // Zed はファイル単体で開いているとき、タスクに ZED_FILE を渡さない（フォルダとして開いたプロジェクトだけ）。
            bail!("編集中ノ ファイルガ 分カリマセン。Zed ノ タスクカラ 呼ンデクダサイ。（ZED_FILE ガ ナク、Zed ノ 状態モ 読メマセン）")
        };
        if !file.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
            bail!("Markdown デハ アリマセン：{}", file.display());
        }
        Ok(file)
    }
}

/// Zed のタスクが置く目印（tasks.json の env）。これがあるときだけ、状態 DB で補ってよい。
pub const FROM_ZED: &str = "PON_FROM_ZED";

/// Zed が渡した値があればそれ。無く、Zed のタスクから呼ばれていれば `fallback`（状態 DB）で補う。
/// Stream Deck などからは補わない（たまたま前面にある Zed の選択を拾わないため）。
pub fn choose(from_env: ZedEnv, from_zed: bool, fallback: impl FnOnce() -> Option<ZedEnv>) -> ZedEnv {
    if from_env.file.is_some() || !from_zed {
        return from_env;
    }
    fallback().unwrap_or(from_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_file() -> ZedEnv {
        ZedEnv { file: Some("D:/x/!.md".into()), ..Default::default() }
    }

    #[test]
    fn zed_が渡したならそれを使い_状態_db_は読まない() {
        let env = choose(with_file(), true, || panic!("読まない"));
        assert_eq!(env, with_file());
    }

    #[test]
    fn zed_のタスクから呼ばれて何も渡っていなければ状態_db_で補う() {
        let env = choose(ZedEnv::default(), true, || Some(with_file()));
        assert_eq!(env, with_file());
    }

    #[test]
    fn zed_のタスク以外_stream_deck_などからは状態_db_を読まない() {
        let env = choose(ZedEnv::default(), false, || panic!("読まない"));
        assert_eq!(env, ZedEnv::default());
    }
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> ZedEnv {
        let m: HashMap<&str, &str> = pairs.iter().copied().collect();
        ZedEnv::read(|k| m.get(k).map(|v| v.to_string()))
    }

    #[test]
    fn 値を読む() {
        let e = env(&[
            ("ZED_FILE", "D:/x/!.md"),
            ("ZED_ROW", "5"),
            ("ZED_COLUMN", "3"),
            ("ZED_SELECTED_TEXT", "neko"),
        ]);
        assert_eq!(e.file, Some(PathBuf::from("D:/x/!.md")));
        assert_eq!((e.row, e.column), (Some(5), Some(3)));
        assert_eq!(e.selected, "neko");
    }

    #[test]
    fn 無ければ_none_と空() {
        assert_eq!(env(&[]), ZedEnv::default());
        assert_eq!(env(&[("ZED_ROW", "abc")]).row, None);
        assert_eq!(env(&[("ZED_FILE", "")]).file, None);
    }

    #[test]
    fn md_file_は_md_だけ通す() {
        assert!(env(&[("ZED_FILE", "D:/x/!.md")]).md_file().is_ok());
        assert!(env(&[("ZED_FILE", "D:/x/a.txt")]).md_file().is_err());
        assert!(env(&[]).md_file().is_err());
    }
}
