//! `![](新,旧)` の行から、画像の改名と本文の書き換えを決める。

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{bail, Result};
use regex::Regex;

use crate::config::{self, Config};
use crate::images;
use crate::names::{is_valid_stem, split_ext};
use crate::zed_env::ZedEnv;

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
        bail!("{} ト {} ガ 両方 アリマス。対象ヲ 特定デキマセン。", pair.a, pair.b);
    }
    if !a_exists && !b_exists {
        bail!("{} モ {} モ 見ツカリマセン。", pair.a, pair.b);
    }
    let (old_name, candidate) = if a_exists { (&pair.a, &pair.b) } else { (&pair.b, &pair.a) };
    let (_, ext) = split_ext(old_name);
    let (stem, cand_ext) = split_ext(candidate);
    if !cand_ext.is_empty() && cand_ext != ext {
        bail!("拡張子ガ 違イマス（{candidate} ← {old_name}）。変換ハ シマセン。");
    }
    if !is_valid_stem(stem) {
        bail!("使エル文字ハ 英数字ト - _ ノミデス：{stem}");
    }
    let new_name = format!("{stem}{ext}");
    if exists(&new_name) {
        bail!("{new_name} ハ 既ニ アリマス。");
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

pub fn run(env: &ZedEnv) -> Result<String> {
    let md = env.md_file()?;
    let found = config::find(md)?;
    run_with(env, found.as_ref().map(|f| &f.config))
}

pub fn run_with(env: &ZedEnv, cfg: Option<&Config>) -> Result<String> {
    let md = env.md_file()?;
    let md_dir = md.parent().unwrap_or(Path::new("."));
    let content = fs::read_to_string(md)?;

    let target = if env.selected.trim().is_empty() {
        line_at(&content, env.row.unwrap_or(0)).to_string()
    } else {
        env.selected.clone()
    };
    let pairs = find_pairs(&target);
    if pairs.is_empty() {
        bail!("コノ行ニ `![](新シイ名前,今ノ名前)` ノ 形ガ アリマセン。");
    }

    let dest_dir = images::dir(cfg, md);
    let exists = |name: &str| md_dir.join(name).exists();
    let plans: Vec<Plan> = pairs.iter().map(|p| resolve(p, exists)).collect::<Result<_>>()?;
    let mut seen = HashSet::new();
    for plan in &plans {
        if !seen.insert(&plan.new_name) {
            bail!("同ジ 新シイ名前ガ 2回 アリマス：{}", plan.new_name);
        }
        if dest_dir.join(&plan.new_name).exists() {
            bail!("{} ハ 既ニ アリマス。", dest_dir.join(&plan.new_name).display());
        }
    }

    // ここから先でだけディスクに触る。
    fs::create_dir_all(&dest_dir)?;
    let mut out = content;
    let mut lines = Vec::new();
    for plan in &plans {
        let dest = dest_dir.join(&plan.new_name);
        fs::rename(md_dir.join(&plan.old_name), &dest)?;
        let link = images::link_for(md, &dest)?;
        out = rewrite_markdown(&out, &plan.old_name, &link);
        lines.push(format!("{} → {}", plan.old_name, link));
    }
    fs::write(md, out)?;
    Ok(lines.join("\n"))
}

fn line_at(content: &str, row: usize) -> &str {
    content.split('\n').nth(row.wrapping_sub(1)).unwrap_or("").trim_end_matches('\r')
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
        assert!(resolve(&pair("neko", "image.png"), exists_in(&[])).unwrap_err().to_string().contains("見ツカリマセン"));
        assert!(resolve(&pair("a.png", "b.png"), exists_in(&["a.png", "b.png"])).unwrap_err().to_string().contains("両方"));
        assert!(resolve(&pair("neko", "image.png"), exists_in(&["image.png", "neko.png"])).unwrap_err().to_string().contains("既ニ アリマス"));
        assert!(resolve(&pair("ねこ", "image.png"), exists_in(&["image.png"])).unwrap_err().to_string().contains("使エル文字"));
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

    use crate::zed_env::ZedEnv;
    use std::fs;
    use std::path::PathBuf;

    fn env_at(file: PathBuf, row: usize, selected: &str) -> ZedEnv {
        ZedEnv { file: Some(file), row: Some(row), column: Some(1), selected: selected.into() }
    }

    #[test]
    fn 行の_新_旧_でファイルを改名し参照を直す() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "---\ntitle: t\n---\n\n![](neko,image.png)\n\n![alt](image.png \"cap\")\n").unwrap();
        fs::write(t.path().join("image.png"), b"PNG").unwrap();

        let msg = run_with(&env_at(md.clone(), 5, ""), None).unwrap();

        assert_eq!(msg, "image.png → neko.png");
        assert!(t.path().join("neko.png").exists());
        assert!(!t.path().join("image.png").exists());
        assert_eq!(fs::read_to_string(&md).unwrap(), "---\ntitle: t\n---\n\n![](neko.png)\n\n![alt](neko.png \"cap\")\n");
    }

    #[test]
    fn 設定の置き場へ動かし_リンクは相対になる() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("2026-10-13.md");
        fs::write(&md, "![](neko,image.png)\n").unwrap();
        fs::write(t.path().join("image.png"), b"PNG").unwrap();
        let cfg = Config::parse("[images]\ndir = 'images/{stem}'\n").unwrap();

        run_with(&env_at(md.clone(), 1, ""), Some(&cfg)).unwrap();

        assert!(t.path().join("images/2026-10-13/neko.png").exists());
        assert_eq!(fs::read_to_string(&md).unwrap(), "![](images/2026-10-13/neko.png)\n");
    }

    #[test]
    fn 選択があればその中の全行() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        let body = "![](neko,image.png)\n![](inu, image_1.png)\n";
        fs::write(&md, body).unwrap();
        fs::write(t.path().join("image.png"), b"").unwrap();
        fs::write(t.path().join("image_1.png"), b"").unwrap();

        run_with(&env_at(md.clone(), 1, body), None).unwrap();

        assert_eq!(fs::read_to_string(&md).unwrap(), "![](neko.png)\n![](inu.png)\n");
    }

    #[test]
    fn 検査に落ちたら何も変えない() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        let body = "![](neko,image.png)\n![](x,missing.png)\n";
        fs::write(&md, body).unwrap();
        fs::write(t.path().join("image.png"), b"").unwrap();

        assert!(run_with(&env_at(md.clone(), 1, body), None).is_err());

        assert!(t.path().join("image.png").exists());
        assert_eq!(fs::read_to_string(&md).unwrap(), body);
    }

    #[test]
    fn 行がはみ出していれば対象なし() {
        let t = tempfile::tempdir().unwrap();
        let md = t.path().join("!.md");
        fs::write(&md, "a\n").unwrap();
        let e = run_with(&env_at(md, 99, ""), None).unwrap_err();
        assert!(e.to_string().contains("形ガ アリマセン"));
    }
}
