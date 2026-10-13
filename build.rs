fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            // 失敗のメッセージ窓のアイコン（win.rs の ERROR_ICON_ID と合わせる）
            .set_icon_with_id("assets/icon-error.ico", "2")
            .compile()
            .expect("アイコンの埋め込みに失敗");
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon-error.ico");
}
