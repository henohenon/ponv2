//! `zed` CLI でファイルを開く。GUI プロセスからなので、コンソールの窓を出さない。
//!
//! ファイルだけを渡すと、Zed はそれを「ファイル 1 つのワークスペース」として開く。
//! その状態では Zed がタスクに ZED_FILE などを渡さず（フォルダとして開いたプロジェクトに属さないため）、
//! Zed 本体の画像貼り付けも黙って失敗する。なので必ずプロジェクトのフォルダも一緒に渡す。

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `zed` に渡す引数。プロジェクトのフォルダ、開くファイルの順。
pub fn open_args(root: &Path, file: &Path) -> Vec<PathBuf> {
    vec![root.to_path_buf(), file.to_path_buf()]
}

pub fn open(root: &Path, file: &Path) -> Result<()> {
    let status = Command::new("zed")
        .args(open_args(root, file))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .context("zed ヲ 起動デキマセン。（PATH ニ zed ガ アリマセン）")?;
    if !status.success() {
        bail!("zed ガ 失敗シマシタ：{status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn フォルダとファイルを両方渡す() {
        let args = open_args(Path::new("D:/notes"), Path::new("D:/notes/2026-10-13.md"));
        assert_eq!(args, vec![PathBuf::from("D:/notes"), PathBuf::from("D:/notes/2026-10-13.md")]);
    }
}
