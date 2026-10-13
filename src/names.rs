//! 画像とページの名前。URL と `_astro/<name>.<hash>.webp` に出るので英数字と - _ に限る。

use std::path::Path;

pub fn is_valid_stem(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `image.png` → (`image`, `.png`)。拡張子が無ければ空。
pub fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Zed と同じ付け方。`image.png`、無ければ `image_1.png`、`image_2.png`、…
pub fn next_free(dir: &Path, base: &str, ext: &str) -> String {
    let first = format!("{base}{ext}");
    if !dir.join(&first).exists() {
        return first;
    }
    (1..)
        .map(|n| format!("{base}_{n}{ext}"))
        .find(|name| !dir.join(name).exists())
        .expect("空きが見つかる")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn 使える文字は英数字と_ハイフン_アンダースコア() {
        assert!(is_valid_stem("neko"));
        assert!(is_valid_stem("neko-01_b"));
        assert!(!is_valid_stem(""));
        assert!(!is_valid_stem("ねこ"));
        assert!(!is_valid_stem("my cat"));
        assert!(!is_valid_stem("a.b"));
        assert!(!is_valid_stem("a/b"));
    }

    #[test]
    fn 拡張子を分ける() {
        assert_eq!(split_ext("image.png"), ("image", ".png"));
        assert_eq!(split_ext("neko"), ("neko", ""));
        assert_eq!(split_ext("a.b.png"), ("a.b", ".png"));
    }

    #[test]
    fn 連番は_image_image_1_image_2() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(next_free(t.path(), "image", ".png"), "image.png");
        fs::write(t.path().join("image.png"), b"").unwrap();
        assert_eq!(next_free(t.path(), "image", ".png"), "image_1.png");
        fs::write(t.path().join("image_1.png"), b"").unwrap();
        assert_eq!(next_free(t.path(), "image", ".png"), "image_2.png");
    }
}
