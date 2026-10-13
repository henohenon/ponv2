fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("アイコンの埋め込みに失敗");
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
