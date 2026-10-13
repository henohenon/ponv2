//! 画像をどこに置き、md からどう書くか。

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::config::Config;

pub fn dir(config: Option<&Config>, md: &Path) -> PathBuf {
    let md_dir = md.parent().unwrap_or(Path::new("."));
    match config.and_then(|c| c.images.dir.as_deref()) {
        None => md_dir.to_path_buf(),
        Some(pattern) => {
            let stem = md.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            md_dir.join(pattern.replace("{stem}", stem))
        }
    }
}

pub fn link_for(md: &Path, image: &Path) -> Result<String> {
    let md_dir = md.parent().unwrap_or(Path::new("."));
    let Some(rel) = pathdiff::diff_paths(image, md_dir) else {
        bail!("{} カラ {} ヘノ 相対パスヲ 作レマセン。", md_dir.display(), image.display())
    };
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(dir: &str) -> Config {
        Config::parse(&format!("[images]\ndir = '{dir}'\n")).unwrap()
    }

    #[test]
    fn 設定が無ければ_md_と同じフォルダ() {
        assert_eq!(dir(None, Path::new("D:/p/!/foo/!.md")), PathBuf::from("D:/p/!/foo"));
    }

    #[test]
    fn stem_を展開して_md_のフォルダからの相対() {
        let c = cfg("images/{stem}");
        assert_eq!(
            dir(Some(&c), Path::new("D:/notes/2026-10-13.md")),
            PathBuf::from("D:/notes/images/2026-10-13")
        );
    }

    #[test]
    fn リンクは_md_のフォルダからの相対で_posix() {
        let md = Path::new("D:/notes/2026-10-13.md");
        assert_eq!(link_for(md, Path::new("D:/notes/images/2026-10-13/neko.png")).unwrap(), "images/2026-10-13/neko.png");
        assert_eq!(link_for(md, Path::new("D:/notes/neko.png")).unwrap(), "neko.png");
        assert_eq!(link_for(Path::new("D:/notes/Kiosk/a.md"), Path::new("D:/notes/images/a/x.png")).unwrap(), "../images/a/x.png");
    }
}
