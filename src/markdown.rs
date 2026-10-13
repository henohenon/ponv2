//! Zed のカーソル位置（1 始まりの行と、行内の UTF-8 バイト位置）に文字列を入れる。

use anyhow::{bail, Result};

pub fn insert_at(content: &str, row: usize, column: usize, selected: &str, text: &str) -> Result<String> {
    if row == 0 || column == 0 {
        bail!("ギョウ マタハ レツガ 0 デス。（1 カラ カゾエマス）");
    }
    // 行の先頭のバイト位置を探す。
    let mut start = 0usize;
    for _ in 1..row {
        match content[start..].find('\n') {
            Some(i) => start += i + 1,
            None => bail!("{row} ギョウメガ アリマセン。"),
        }
    }
    let line_end = content[start..].find('\n').map_or(content.len(), |i| start + i);
    let line_len = content[start..line_end].trim_end_matches('\r').len();
    let col = column - 1;
    if col > line_len {
        bail!("{row} ギョウメニ {column} レツハ アリマセン。");
    }
    let pos = start + col;
    if !content.is_char_boundary(pos) {
        bail!("{row} ギョウメ {column} レツハ モジノ トチュウ デス。");
    }
    let end = if !selected.is_empty() && content[pos..].starts_with(selected) {
        pos + selected.len()
    } else {
        pos
    };
    let mut out = String::with_capacity(content.len() + text.len());
    out.push_str(&content[..pos]);
    out.push_str(text);
    out.push_str(&content[end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn カーソル位置に入れる() {
        assert_eq!(insert_at("ab\ncd\n", 2, 2, "", "X").unwrap(), "ab\ncXd\n");
        assert_eq!(insert_at("ab\ncd\n", 2, 3, "", "X").unwrap(), "ab\ncdX\n");
        assert_eq!(insert_at("ab\n", 1, 1, "", "X").unwrap(), "Xab\n");
    }

    #[test]
    fn 選択があれば置き換える() {
        assert_eq!(insert_at("say neko now\n", 1, 5, "neko", "![](neko.png)").unwrap(), "say ![](neko.png) now\n");
    }

    #[test]
    fn 選択がその位置に無ければ挿入にする() {
        assert_eq!(insert_at("say neko now\n", 1, 5, "inu", "X").unwrap(), "say Xneko now\n");
    }

    #[test]
    fn 日本語の前にある列を正しく扱う() {
        // 「ねこ」は 6 バイト。列 7 はその直後。
        assert_eq!(insert_at("ねこ\n", 1, 7, "", "X").unwrap(), "ねこX\n");
        assert_eq!(insert_at("ねこ\n", 1, 4, "", "X").unwrap(), "ねXこ\n");
    }

    #[test]
    fn 文字の途中の列は_err() {
        assert!(insert_at("ねこ\n", 1, 2, "", "X").is_err());
    }

    #[test]
    fn crlf_を保つ() {
        assert_eq!(insert_at("ab\r\ncd\r\n", 2, 3, "", "X").unwrap(), "ab\r\ncdX\r\n");
    }

    #[test]
    fn 行がはみ出していれば_err() {
        assert!(insert_at("ab\n", 3, 1, "", "X").is_err());
        assert!(insert_at("ab\n", 1, 9, "", "X").is_err());
    }
}
