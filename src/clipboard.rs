//! クリップボードの画像を PNG のバイト列にする。薄い殻。単体テストは無し（手で確かめる）。
//!
//! クリップボードに PNG 形式があればそのバイト列をそのまま使い（再圧縮しない）、
//! 無ければビットマップ（DIB）を読んで PNG に変換する。

use anyhow::{anyhow, Context, Result};

pub fn read_png() -> Result<Vec<u8>> {
    if let Some(png) = raw_png()? {
        return Ok(png);
    }
    dib_as_png()
}

/// 登録形式 "PNG" が載っていれば、そのバイト列。スクショツールや Zed はこれを載せる。
fn raw_png() -> Result<Option<Vec<u8>>> {
    use clipboard_win::{is_format_avail, raw, register_format, Clipboard};

    let Some(fmt) = register_format("PNG") else { return Ok(None) };
    if !is_format_avail(fmt.get()) {
        return Ok(None);
    }
    let _clip = Clipboard::new_attempts(10).map_err(|e| anyhow!("クリップボードを開けなかった：{e}"))?;
    let mut buf = Vec::new();
    raw::get_vec(fmt.get(), &mut buf).map_err(|e| anyhow!("クリップボードを読めなかった：{e}"))?;
    Ok(if buf.is_empty() { None } else { Some(buf) })
}

fn dib_as_png() -> Result<Vec<u8>> {
    let mut cb = arboard::Clipboard::new().context("クリップボードを開けなかった")?;
    let img = match cb.get_image() {
        Ok(img) => img,
        Err(arboard::Error::ContentNotAvailable) => return Err(anyhow!("クリップボードに画像が無いよ")),
        Err(e) => return Err(anyhow!("クリップボードを読めなかった：{e}")),
    };
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width as u32, img.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        // 速さ優先。ビルド側が webp に変換し直すので、ここで縮めても意味が薄い。
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().context("PNG のヘッダを書けなかった")?;
        w.write_image_data(&img.bytes).context("PNG を書けなかった")?;
    }
    Ok(out)
}
