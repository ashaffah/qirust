//! # qirust
//!
//! A Rust library for generating QR codes with customizable rendering and styling options.
//!
//! `qirust` is a safe and efficient library for encoding text or binary data into QR codes, adhering
//! to the QR Code Model 2 specification. It supports versions 1 to 40, four error correction levels
//! (Low, Medium, Quartile, High), and multiple output formats including ASCII art, PNG images, SVGs,
//! and in-memory image buffers. The library provides advanced styling features such as logo embedding,
//! custom colors, and square or rounded frames, optimized for performance with techniques like
//! horizontal module grouping and caching.
//!
//! ## Features
//!
//! - **Encoding Modes**: Supports numeric, alphanumeric, byte, and ECI modes (Kanji mode defined but
//!   not implemented).
//! - **Error Correction**: Four levels (Low, Medium, Quartile, High) to balance data capacity and
//!   robustness.
//! - **Output Formats**: Render QR codes as console ASCII art, PNG images, SVGs, or in-memory image
//!   buffers.
//! - **Styling Options**: Embed logos, customize colors, and apply square or rounded frames behind
//!   logos.
//! - **Performance**: Optimized with horizontal module grouping, logo caching, and minimal memory
//!   allocations.
//! - **Safety**: Pure Rust implementation with no unsafe code, ensuring memory safety and reliability.
//!
//! ## Installation
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! qirust = "0.1"
//! image = "0.25"
//! ```
//!
//! ## Examples
//!
//! Generate a styled QR code with a logo and rounded frame:
//!
//! ```rust,no_run
//! use qirust::helper::{generate_frameqr, FrameQrConfig, FrameStyle};
//! use qirust::qrcode::QrCodeEcc;
//!
//! let config = FrameQrConfig::new("logo.png").unwrap()
//!     .with_ecc(QrCodeEcc::High)
//!     .with_upscale(6).unwrap()
//!     .with_directory("output")
//!     .with_filename("styled_qr")
//!     .with_color([255, 165, 0])
//!     .with_outer_frame(40)
//!     .with_inner_frame(10)
//!     .with_frame_style(FrameStyle::Rounded);
//!
//! generate_frameqr("https://example.com", config)
//!     .expect("Failed to generate QR code");
//! ```
//!
//! Generate an in-memory image buffer for a basic QR code:
//!
//! ```rust
//! use qirust::helper::{generate_image_buffer, QrConfig};
//!
//! let config = QrConfig::new()
//!     .with_border(4).unwrap()
//!     .with_fg_color([255, 0, 0])
//!     .with_bg_color([255, 255, 255])
//!     .with_scale(6).unwrap();
//!
//! let img = generate_image_buffer("Hello, World!", config)
//!     .expect("Failed to generate image buffer");
//! # img.save("target/qr_lib.png").ok();
//! ```
//!
//! Encode text and print to console:
//!
//! ```rust
//! use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
//! use qirust::helper::print_qr;
//!
//! fn main() -> Result<(), qirust::qrcode::DataTooLong> {
//!     let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
//!     let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
//!     let qr = QrCode::encode_text(
//!         "Hello, World!",
//!         &mut tempbuffer,
//!         &mut outbuffer,
//!         EncodeTextOptions {
//!             ecl: QrCodeEcc::Low,
//!             minversion: Version::MIN,
//!             maxversion: Version::MAX,
//!             mask: None,
//!             boostecl: true,
//!         },
//!     )?;
//!     print_qr(&qr);
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`qrcode`]: Core functionality for encoding QR codes, including data segmentation and error
//!   correction.
//! - [`helper`]: Utilities for rendering QR codes in various formats with styling options.
//!
//! ## Error Handling
//!
//! - [`qrcode::DataTooLong`]: Returned when input data exceeds the QR code's capacity. Handle by
//!   reducing data size, increasing version, or lowering error correction.
//! - [`helper::HelperError`]: Wraps various errors including image processing, I/O, and validation.
//! - [`image::ImageError`]: Occurs during image processing or file I/O (e.g., invalid logo path or
//!   permissions issues).
//!
//! ## Limitations
//!
//! - **Kanji Mode**: Defined but not fully implemented in the [`qrcode`] module.
//! - **ECI Mode**: Supported but requires careful handling for non-standard character sets.
//! - **Logo Size**: Automatically resized to one-third of QR code dimensions for scannability (up to
//!   40% for SVG outputs).
//!
//! ## Performance
//!
//! The library is optimized for efficiency:
//! - **Horizontal Module Grouping**: Reduces rendering complexity in SVG and image outputs, improving
//!   performance for high-version QR codes (e.g., Version 40).
//! - **Logo Caching**: Uses global `Mutex`-based caches for resized logos and base64-encoded images,
//!   minimizing redundant processing.
//! - **Memory Efficiency**: Precomputes buffer sizes and uses minimal allocations for encoding and
//!   rendering.

pub mod helper;
pub mod qrcode;

pub use helper::{
    FrameQrConfig, FrameQrSaveConfig, FrameQrSvgConfig, FrameStyle, HelperError, QrConfig,
};
pub use qrcode::{DataTooLong, EncodeTextOptions, QrCode, QrCodeEcc, Version};
