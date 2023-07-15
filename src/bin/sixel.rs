#[cfg(feature = "image")]
fn main() {
    use std::env;

    let path = env::args()
        .nth(1)
        .expect("Usage: <program> [path/to/image]");

    let dyn_image = image::io::Reader::open(path)
        .expect("Could not open image")
        .decode()
        .expect("Could not decode image");

    let image = dyn_image.into_rgba8();
    let bytes = image.as_raw();

    match sixel_bytes::sixel_string(
        bytes,
        image.width() as _,
        image.height() as _,
        sixel_bytes::PixelFormat::RGBA8888,
        sixel_sys::DiffusionMethod::Stucki,
    ) {
        Err(err) => eprintln!("{err}"),
        Ok(data) => print!("{data}"),
    }
}
