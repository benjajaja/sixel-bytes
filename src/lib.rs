//! Encode an image with [sixel-sys].
//!
//! [sixel-sys]: https://crates.io/crates/sixel-sys
//!
//! This is my first crate that uses `unsafe` and FFI. Please inspect the source code yourself, the
//! crate is very small. PRs are welcome.
//!
//! To write a sixel to a file, [sixel-rs] is probably safer and has more options.
//!
//! [sixel-rs]: https://crates.io/crates/sixel-rs
//!
//! # Examples
//!
//! Encode a generated image to sixel and print it:
//! ```rust
//! let mut bytes: Vec<u8> = Vec::new();
//! for x in 0..255 {
//!     for y in 0..255 {
//!         bytes.append(&mut vec![x, 0, y]);
//!     }
//! }
//!
//! let data = sixel_bytes::sixel_string(
//!     &bytes,
//!     255,
//!     255,
//!     sixel_bytes::PixelFormat::RGB888,
//!     sixel_bytes::DiffusionMethod::Atkinson,
//! ).unwrap();
//! assert_eq!(&data[..3], "\u{1b}Pq");
//! ```
//!
//! Encode an image from the [image] crate to sixel and print it:
//! ```ignore
//! let image = image::io::Reader::open("./assets/Ada.png")
//!     .unwrap()
//!     .decode()
//!     .unwrap()
//!     .into_rgba8();
//! let bytes = image.as_raw();
//!
//! match sixel_bytes::sixel_string(
//!     bytes,
//!     image.width() as _,
//!     image.height() as _,
//!     sixel_bytes::PixelFormat::RGBA8888,
//!     sixel_sys::DiffusionMethod::Stucki,
//! ) {
//!     Err(err) => eprintln!("{err}"),
//!     Ok(data) => print!("{data}"),
//! }
//! ```
//!
//! # Binaries
//!
//! `sixel <path/to/image>` uses the [image] crate to load an image with supported formats, convert
//! to RGBA8888, encode to sixel, and dump the resulting string to stdout. It must be built with
//! the `image` feature.
//!
//! `test-sixel` just generates some 255x255 image with a gradient and dumps it to stdout.
//!
//! # Features
//! The `image` feature is disabled by default but needed for the `sixel` binary.
//!
//! [image]: https://crates.io/crates/image

use core::fmt;
use std::{
    ffi::{c_int, c_uchar, c_void},
    mem, ptr, slice,
    string::FromUtf8Error,
};

pub use sixel_sys::status;
pub use sixel_sys::status::Status;
pub use sixel_sys::DiffusionMethod;
pub use sixel_sys::PixelFormat;
use sixel_sys::{
    sixel_dither_initialize, sixel_dither_new, sixel_dither_set_diffusion_type,
    sixel_dither_set_pixelformat, sixel_encode, sixel_output_destroy, sixel_output_new,
    sixel_output_set_encode_policy, Dither, EncodePolicy, MethodForLargest, Output,
};

#[derive(Debug)]
pub enum SixelError {
    Sixel(Status),
    Utf8(FromUtf8Error),
}

impl fmt::Display for SixelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SixelError::Sixel(status) => write!(f, "Sixel error code {0}", status),
            SixelError::Utf8(utf8_error) => utf8_error.fmt(f),
        }
    }
}

impl SixelError {
    /// This is not exactly [TryFrom] nor [From]: `status::OK` produces `Ok(())`, other statuses
    /// `Err(SixelError)`.
    ///
    /// ```no_run
    /// # use sixel_bytes::{SixelError, Status, status};
    /// # fn some_sixel_sys_function() -> Status {
    /// #     status::ERR
    /// # }
    /// SixelError::from_status(some_sixel_sys_function())?;
    /// # Ok::<(), SixelError>(())
    /// ```
    pub fn from_status(value: c_int) -> Result<(), Self> {
        match value {
            status::OK => Ok(()),
            code => Err(SixelError::Sixel(code)),
        }
    }
}

// According to sixel-sys, this is unused/ignored.
const DEPTH_ALWAYS_IGNORED: i32 = 24;

/// Encode image bytes to a [String] containing the sixel data.
///
/// The `bytes` must match the width, height, and "pixelformat".
pub fn sixel_string(
    bytes: &[u8],
    width: i32,
    height: i32,
    pixelformat: PixelFormat,
    method_for_diffuse: DiffusionMethod,
) -> Result<String, SixelError> {
    let mut sixel_data: Vec<i8> = Vec::new();
    let rust_object_ptr: *mut c_void = &mut sixel_data as *mut _ as *mut c_void;

    let mut output: *mut Output = ptr::null_mut() as *mut _;
    let output_ptr: *mut *mut Output = &mut output as *mut _;

    let mut dither: *mut Dither = ptr::null_mut() as *mut _;
    let dither_ptr: *mut *mut Dither = &mut dither as *mut _;

    let pixels = bytes.as_ptr() as *mut c_uchar;

    unsafe extern "C" fn callback(
        data: *mut ::std::os::raw::c_char,
        size: ::std::os::raw::c_int,
        priv_: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int {
        let sixel_data: &mut Vec<i8> = &mut *(priv_ as *mut Vec<i8>);

        let data_slice: &mut [i8] =
            slice::from_raw_parts_mut(if data.is_null() { return 1 } else { data }, size as usize);
        sixel_data.append(&mut data_slice.to_vec());
        status::OK
    }

    unsafe {
        SixelError::from_status(sixel_output_new(
            output_ptr,
            Some(callback),
            rust_object_ptr,
            ptr::null_mut(),
        ))?;

        sixel_output_set_encode_policy(output, EncodePolicy::Auto);

        SixelError::from_status(sixel_dither_new(dither_ptr, 256, ptr::null_mut()))?;

        sixel_dither_initialize(
            dither,
            pixels,
            width,
            height,
            pixelformat,
            MethodForLargest::Auto,
            sixel_sys::MethodForRepColor::Auto,
            sixel_sys::QualityMode::Auto,
        );
        sixel_dither_set_pixelformat(dither, pixelformat);
        sixel_dither_set_diffusion_type(dither, method_for_diffuse);

        SixelError::from_status(sixel_encode(
            pixels,
            width,
            height,
            DEPTH_ALWAYS_IGNORED,
            dither,
            output,
        ))?;

        sixel_output_destroy(output);

        // TODO: should we just return something like [u8]? Is all sixel data valid utf8?
        String::from_utf8(mem::transmute(sixel_data)).map_err(SixelError::Utf8)
    }
}
