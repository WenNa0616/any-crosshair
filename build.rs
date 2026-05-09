fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let png_data = include_bytes!("crosshair.png");
    let png_len = png_data.len() as u32;

    // ICO format: ICONDIR + ICONDIRENTRY + PNG data
    let mut ico = Vec::with_capacity(6 + 16 + png_len as usize);

    // ICONDIR
    ico.extend_from_slice(&[0, 0]);           // reserved
    ico.extend_from_slice(&[1, 0]);           // type: icon
    ico.extend_from_slice(&[1, 0]);           // count: 1 image

    // ICONDIRENTRY
    ico.push(0);                              // width (0 = 256)
    ico.push(0);                              // height (0 = 256)
    ico.push(0);                              // color palette
    ico.push(0);                              // reserved
    ico.extend_from_slice(&[1, 0]);           // color planes
    ico.extend_from_slice(&[32, 0]);          // bits per pixel
    ico.extend_from_slice(&png_len.to_le_bytes()); // data size
    let offset = 6u32 + 16;                  // data offset
    ico.extend_from_slice(&offset.to_le_bytes());

    // PNG data
    ico.extend_from_slice(png_data);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ico_path = std::path::Path::new(&out_dir).join("icon.ico");
    std::fs::write(&ico_path, &ico).expect("Failed to write ICO");

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_str().unwrap());
    res.compile().unwrap();
}
