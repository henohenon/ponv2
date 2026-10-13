#![windows_subsystem = "windows"]

mod args;
mod clipboard;
mod config;
mod images;
mod markdown;
mod names;
mod open;
mod pages;
mod paste;
mod rename;
mod win;
mod zed;
mod zed_env;

use std::io::Write;

use args::Command;

fn main() {
    win::attach_parent_console();
    win::enable_dpi_awareness();
    let code = match run() {
        Ok(msg) => {
            // コンソールが無い／閉じているときに println! が panic しないよう、失敗は捨てる。
            let _ = writeln!(std::io::stdout(), "{msg}");
            0
        }
        Err(e) => {
            // 失敗の通知はメッセージ窓が本命。コンソールへの書き出しは後回しにし、失敗しても構わない。
            let text = format!("{e:#}");
            win::message_box(&text);
            let _ = writeln!(std::io::stderr(), "{text}");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> anyhow::Result<String> {
    let cmd = args::parse(std::env::args().skip(1))?;
    let env = zed_env::ZedEnv::from_env();
    match cmd {
        Command::Paste => paste::run(&env),
        Command::Rename => rename::run(&env),
        Command::Open { name, today, project } => {
            open::run(name, today, project, &env, || win::input_box("pon — ナマエ"))
        }
    }
}
