//! `zed` CLI でファイルを開く。GUI プロセスからなので、コンソールの窓を出さない。

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn open(path: &Path) -> Result<()> {
    let status = Command::new("zed")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .context("zed を起動できない（PATH に zed が無い）")?;
    if !status.success() {
        bail!("zed が失敗した: {status}");
    }
    Ok(())
}
