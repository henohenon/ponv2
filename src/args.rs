//! サブコマンドと引数。人向けの CLI ではないので、ヘルプや短縮形は持たない。

use std::path::PathBuf;

use anyhow::{bail, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Paste,
    Rename,
    Open {
        name: Option<String>,
        today: bool,
        project: Option<PathBuf>,
    },
}

/// 先頭はサブコマンド名。`open` だけが引数を取る。
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command> {
    let mut it = args.into_iter();
    let Some(sub) = it.next() else {
        bail!("コマンドヲ 指定シテクダサイ。（paste / rename / open）")
    };
    match sub.as_str() {
        "paste" => rest_is_empty(it, Command::Paste),
        "rename" => rest_is_empty(it, Command::Rename),
        "open" => parse_open(it),
        other => bail!("不明ナ コマンドデス：{other}"),
    }
}

fn rest_is_empty(mut it: impl Iterator<Item = String>, cmd: Command) -> Result<Command> {
    match it.next() {
        None => Ok(cmd),
        Some(extra) => bail!("余分ナ 引数デス：{extra}"),
    }
}

fn parse_open(mut it: impl Iterator<Item = String>) -> Result<Command> {
    let mut name = None;
    let mut today = false;
    let mut project = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--today" => today = true,
            "--project" => {
                let Some(dir) = it.next() else {
                    bail!("--project ノ 後ニ フォルダヲ 指定シテクダサイ。")
                };
                project = Some(PathBuf::from(dir));
            }
            s if s.starts_with("--") => bail!("不明ナ 引数デス：{s}"),
            s => {
                if name.is_some() {
                    bail!("名前ガ 2ツ アリマス：{s}");
                }
                name = Some(s.to_string());
            }
        }
    }
    Ok(Command::Open { name, today, project })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Command {
        parse(s.split_whitespace().map(str::to_string)).unwrap()
    }

    #[test]
    fn paste_と_rename() {
        assert_eq!(p("paste"), Command::Paste);
        assert_eq!(p("rename"), Command::Rename);
    }

    #[test]
    fn open_は名前_today_project_を取る() {
        assert_eq!(
            p("open"),
            Command::Open { name: None, today: false, project: None }
        );
        assert_eq!(
            p("open アイデア --project D:/notes"),
            Command::Open {
                name: Some("アイデア".into()),
                today: false,
                project: Some(PathBuf::from("D:/notes")),
            }
        );
        assert_eq!(
            p("open --today"),
            Command::Open { name: None, today: true, project: None }
        );
    }

    #[test]
    fn 知らないコマンドや引数は_err() {
        assert!(parse(["pon".to_string()]).is_err());
        assert!(parse(Vec::<String>::new()).is_err());
        assert!(parse(["open".to_string(), "--project".to_string()]).is_err());
        assert!(parse(["paste".to_string(), "x".to_string()]).is_err());
    }
}
