//! クリップボードの画像を PNG のバイト列にする。薄い殻。単体テストは無し（手で確かめる）。

use anyhow::{anyhow, Context, Result};

pub fn read_png() -> Result<Vec<u8>> {
    let mut cb = arboard::Clipboard::new().context("クリップボードを開けない")?;
    let img = match cb.get_image() {
        Ok(img) => img,
        Err(arboard::Error::ContentNotAvailable) => return Err(anyhow!("クリップボードに画像が無い")),
        Err(e) => return Err(anyhow!("クリップボードを読めない: {e}")),
    };
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width as u32, img.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().context("PNG のヘッダを書けない")?;
        w.write_image_data(&img.bytes).context("PNG を書けない")?;
    }
    Ok(out)
}
