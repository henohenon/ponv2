#![windows_subsystem = "windows"]

mod args;
mod clipboard;
mod config;
mod images;
mod log;
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
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let env = zed_env::ZedEnv::from_env();
    let stamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
    let result = run(&argv, &env);
    let (code, outcome) = match &result {
        Ok(msg) => (0, format!("ok: {msg}")),
        Err(e) if e.downcast_ref::<open::Cancelled>().is_some() => (0, "cancelled".to_string()),
        Err(e) => (1, format!("err: {e:#}")),
    };
    // 窓を出す前に記録する（窓が開いたまま放置されても、呼ばれたことは残る）。
    log::write(&log::entry(&stamp, &argv, &env, &outcome));
    match result {
        Ok(msg) => {
            // コンソールが無い／閉じているときに println! が panic しないよう、失敗は捨てる。
            let _ = writeln!(std::io::stdout(), "{msg}");
        }
        Err(e) if e.downcast_ref::<open::Cancelled>().is_some() => {}
        Err(e) => {
            // 失敗の通知はメッセージ窓が本命。コンソールへの書き出しは後回しにし、失敗しても構わない。
            let text = format!("{e:#}");
            win::message_box(&text);
            let _ = writeln!(std::io::stderr(), "{text}");
        }
    }
    std::process::exit(code);
}

fn run(argv: &[String], env: &zed_env::ZedEnv) -> anyhow::Result<String> {
    let cmd = args::parse(argv.iter().cloned())?;
    match cmd {
        Command::Paste => paste::run(env),
        Command::Rename => rename::run(env),
        Command::Open { name, today, project } => {
            open::run(name, today, project, env, || win::input_box("pon — 名前"))
        }
    }
}
