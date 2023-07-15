fn main() {
    let mut bytes: Vec<u8> = Vec::new();
    for x in 0..255 {
        for y in 0..255 {
            bytes.append(&mut vec![x, 0, y]);
        }
    }

    match sixel_bytes::sixel_string(
        &bytes,
        255,
        255,
        sixel_bytes::PixelFormat::RGB888,
        sixel_sys::DiffusionMethod::None,
    ) {
        Err(err) => eprintln!("{err}"),
        Ok(data) => print!("{data}"),
    }
}
