//! `![](新,旧)` の行から、画像の改名と本文の書き換えを決める。

use std::sync::LazyLock;

use anyhow::{bail, Result};
use regex::Regex;

use crate::names::{is_valid_stem, split_ext};

#[derive(Debug, PartialEq, Eq)]
pub struct Pair {
    pub a: String,
    pub b: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub old_name: String,
    pub new_name: String,
}

/// `![alt](x , y)` の x と y。名前には空白・カンマ・括弧は入らない。
static PAIR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!\[[^\]]*\]\(\s*([^\s,()]+)\s*,\s*([^\s,()]+)\s*\)").unwrap());

pub fn find_pairs(text: &str) -> Vec<Pair> {
    PAIR_RE
        .captures_iter(text)
        .map(|c| Pair { a: c[1].to_string(), b: c[2].to_string() })
        .collect()
}

/// どちらが旧名かは「存在する方」。新名は拡張子を補って整える。
pub fn resolve(pair: &Pair, exists: impl Fn(&str) -> bool) -> Result<Plan> {
    let (a_exists, b_exists) = (exists(&pair.a), exists(&pair.b));
    if a_exists && b_exists {
        bail!("{} と {} の両方が既にある。どちらを改名するのか決められない", pair.a, pair.b);
    }
    if !a_exists && !b_exists {
        bail!("{} も {} も見つからない", pair.a, pair.b);
    }
    let (old_name, candidate) = if a_exists { (&pair.a, &pair.b) } else { (&pair.b, &pair.a) };
    let (_, ext) = split_ext(old_name);
    let (stem, cand_ext) = split_ext(candidate);
    if !cand_ext.is_empty() && cand_ext != ext {
        bail!("拡張子が違う（{candidate} ← {old_name}）。形式の変換はしない");
    }
    if !is_valid_stem(stem) {
        bail!("使える文字は英数字と - _ だけ: {stem}");
    }
    let new_name = format!("{stem}{ext}");
    if exists(&new_name) {
        bail!("{new_name} は既にある");
    }
    Ok(Plan { old_name: old_name.clone(), new_name })
}

/// 「新,旧」の行は新しいリンクだけにし、旧名を指す他の参照（`](旧)` `](./旧 "cap")`）も直す。
pub fn rewrite_markdown(content: &str, old_name: &str, new_link: &str) -> String {
    let old = regex::escape(old_name);
    let name = r"[^\s,()]+";
    let pair_re = Regex::new(&format!(
        r"(!\[[^\]]*\]\()\s*(?:{name}\s*,\s*{old}|{old}\s*,\s*{name})\s*\)"
    ))
    .unwrap();
    // regex crate は先読みを持たないので、後ろの 1 文字（空白か `)`）を捕捉して戻す。
    let ref_re = Regex::new(&format!(r"(\]\(\s*)(?:\./)?{old}([\s)])")).unwrap();
    let link = new_link.replace('$', "$$");
    let out = pair_re.replace_all(content, format!("${{1}}{link})").as_str());
    ref_re.replace_all(&out, format!("${{1}}{link}${{2}}").as_str()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: &str, b: &str) -> Pair {
        Pair { a: a.into(), b: b.into() }
    }
    fn exists_in<'a>(names: &'a [&str]) -> impl Fn(&str) -> bool + 'a {
        move |n| names.contains(&n)
    }

    #[test]
    fn 括弧の中の_新_旧_を拾う() {
        assert_eq!(find_pairs("![](neko,image.png)"), vec![pair("neko", "image.png")]);
        assert_eq!(find_pairs("![](neko , image.png)"), vec![pair("neko", "image.png")]);
    }

    #[test]
    fn 複数行から全部拾う() {
        let t = "本文\n![](neko,image.png)\n![](inu,image_1.png)";
        assert_eq!(find_pairs(t), vec![pair("neko", "image.png"), pair("inu", "image_1.png")]);
    }

    #[test]
    fn カンマの無い画像や普通のリンクは拾わない() {
        assert_eq!(find_pairs("![](image.png) と [a,b](http://x/y,z)"), vec![]);
    }

    #[test]
    fn 存在する方が旧名_無い方が新名() {
        let p = resolve(&pair("neko", "image.png"), exists_in(&["image.png"])).unwrap();
        assert_eq!(p, Plan { old_name: "image.png".into(), new_name: "neko.png".into() });
        let p = resolve(&pair("image.png", "neko"), exists_in(&["image.png"])).unwrap();
        assert_eq!(p, Plan { old_name: "image.png".into(), new_name: "neko.png".into() });
    }

    #[test]
    fn 新名の拡張子は省けて_書けば同じでなければならない() {
        let p = resolve(&pair("neko.png", "image.png"), exists_in(&["image.png"])).unwrap();
        assert_eq!(p.new_name, "neko.png");
        let e = resolve(&pair("neko.jpg", "image.png"), exists_in(&["image.png"])).unwrap_err();
        assert!(e.to_string().contains("拡張子"));
    }

    #[test]
    fn 両方無い_両方ある_既にある_使えない文字() {
        assert!(resolve(&pair("neko", "image.png"), exists_in(&[])).unwrap_err().to_string().contains("見つからない"));
        assert!(resolve(&pair("a.png", "b.png"), exists_in(&["a.png", "b.png"])).unwrap_err().to_string().contains("両方"));
        assert!(resolve(&pair("neko", "image.png"), exists_in(&["image.png", "neko.png"])).unwrap_err().to_string().contains("既にある"));
        assert!(resolve(&pair("ねこ", "image.png"), exists_in(&["image.png"])).unwrap_err().to_string().contains("使える文字"));
    }

    #[test]
    fn 新_旧_の行を新しいリンクだけにする() {
        assert_eq!(rewrite_markdown("![](neko,image.png)", "image.png", "neko.png"), "![](neko.png)");
        assert_eq!(rewrite_markdown("![](neko , image.png)", "image.png", "neko.png"), "![](neko.png)");
        assert_eq!(rewrite_markdown("![](image.png,neko)", "image.png", "neko.png"), "![](neko.png)");
    }

    #[test]
    fn 旧名を指す他の参照も直す_置き場が変わる形でも() {
        let md = "![](neko,image.png)\n\n![alt](image.png \"キャプション\")\n[![](./image.png)](https://x/)";
        let want = "![](images/2026-10-13/neko.png)\n\n![alt](images/2026-10-13/neko.png \"キャプション\")\n[![](images/2026-10-13/neko.png)](https://x/)";
        assert_eq!(rewrite_markdown(md, "image.png", "images/2026-10-13/neko.png"), want);
    }

    #[test]
    fn 前方一致するだけの別ファイルは触らない() {
        let md = "![](image.png.bak) ![](image.png2)";
        assert_eq!(rewrite_markdown(md, "image.png", "neko.png"), md);
    }

    #[test]
    fn 改行コードを変えない() {
        assert_eq!(rewrite_markdown("a\r\n![](neko,image.png)\r\nb", "image.png", "neko.png"), "a\r\n![](neko.png)\r\nb");
    }
}
