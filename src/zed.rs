//! `zed` CLI でファイルを開く。GUI プロセスからなので、コンソールの窓を出さない。
//!
//! ファイルだけを渡す（ファイル単体で開くのが本人の好み）。単体で開いた Zed はタスクに ZED_FILE などを
//! 渡さないが、pon は Zed の状態 DB で補う（zed_state.rs）。

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `zed` に渡す引数。
pub fn open_args(file: &Path) -> Vec<PathBuf> {
    vec![file.to_path_buf()]
}

pub fn open(file: &Path) -> Result<()> {
    let status = Command::new("zed")
        .args(open_args(file))
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
    fn ファイルだけを渡す() {
        // ファイル単体で開く（本人の好み）。単体だとタスクに ZED_FILE が渡らないが、pon は状態 DB で補う。
        let args = open_args(Path::new("D:/notes/2026-10-13.md"));
        assert_eq!(args, vec![PathBuf::from("D:/notes/2026-10-13.md")]);
    }
}
