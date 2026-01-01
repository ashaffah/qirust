/// Utilities for rendering QR codes.
///
/// This module provides functions to render [QrCode]s as console output, PNG images, SVGs, or
/// in-memory image buffers. It supports advanced styling options, including logo embedding, custom
/// colors, and square or rounded frames. The implementation is optimized for performance with
/// features like horizontal module grouping and caching, and it is written in safe, pure Rust
/// without external dependencies.
///
/// # Features
///
/// - Render QR codes in multiple formats: ASCII art, PNG, SVG, and in-memory buffers.
/// - Support styling with logos, custom colors, and square or rounded frames.
/// - Optimized for performance with horizontal module grouping and caching for logo processing.
/// - Safe and pure Rust implementation with no unsafe code.
///
/// # Examples
///
/// Generate a basic QR code as an in-memory image buffer:
///
/// ```rust
/// use qirust::helper::{generate_image_buffer, QrConfig};
///
/// let config = QrConfig::new()
///     .with_border(4).unwrap()
///     .with_scale(4).unwrap();
/// let img = generate_image_buffer("Hello, World!", config)
///     .expect("Failed to generate image buffer");
/// # img.save("target/qr_example.png").ok();
/// ```
///
/// Generate a styled QR code with a logo and rounded frame:
///
/// ```rust,no_run
/// use qirust::helper::{generate_frameqr, FrameQrConfig, FrameStyle};
/// use qirust::qrcode::QrCodeEcc;
///
/// let config = FrameQrConfig::new("src/logo.png").unwrap()
///     .with_ecc(QrCodeEcc::High)
///     .with_upscale(6).unwrap()
///     .with_directory("output")
///     .with_filename("styled_qr")
///     .with_color([255, 165, 0])
///     .with_outer_frame(40)
///     .with_inner_frame(10)
///     .with_frame_style(FrameStyle::Rounded);
///
/// generate_frameqr("https://example.com", config)
///     .expect("Failed to generate QR code");
/// ```
use crate::qrcode::{DataTooLong, EncodeTextOptions, QrCode, QrCodeEcc, Version};
use image::{
    imageops::{overlay, replace, resize, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, Luma, Rgb, Rgba, RgbaImage,
};
use std::{
    env,
    error::Error,
    fmt,
    fmt::Write,
    fs,
    io::Write as IoWrite,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

// Constants for avoiding magic numbers
const MAX_UPSCALE_FACTOR: u32 = 100;
const MAX_BORDER_SIZE: u32 = 200;
const MAX_IMAGE_DIMENSION: u32 = 50000;
const DEFAULT_UPSCALE_FACTOR: u32 = 8;
const DEFAULT_BORDER_SIZE: u32 = 4;
const DEFAULT_SCALE: u32 = 4;
const LOGO_SIZE_DIVISOR: u32 = 3;
const DEFAULT_INNER_FRAME: u32 = 3;

/// Custom error type untuk operasi helper QR code
#[derive(Debug)]
pub enum HelperError {
    ImageError(image::ImageError),
    DataTooLong(DataTooLong),
    IoError(std::io::Error),
    InvalidInput(String),
}

impl fmt::Display for HelperError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HelperError::ImageError(e) => write!(f, "Image error: {}", e),
            HelperError::DataTooLong(e) => write!(f, "Data too long: {:?}", e),
            HelperError::IoError(e) => write!(f, "IO error: {}", e),
            HelperError::InvalidInput(s) => write!(f, "Invalid input: {}", s),
        }
    }
}

impl Error for HelperError {}

impl From<image::ImageError> for HelperError {
    fn from(err: image::ImageError) -> Self {
        HelperError::ImageError(err)
    }
}

impl From<DataTooLong> for HelperError {
    fn from(err: DataTooLong) -> Self {
        HelperError::DataTooLong(err)
    }
}

impl From<std::io::Error> for HelperError {
    fn from(err: std::io::Error) -> Self {
        HelperError::IoError(err)
    }
}

/// Configuration for basic QR code rendering.
#[derive(Debug, Clone)]
pub struct QrConfig {
    /// Border size in modules (defaults to 4)
    pub border: u32,
    /// Foreground color as RGB (defaults to black [0, 0, 0])
    pub fg_color: [u8; 3],
    /// Background color as RGB (defaults to white [255, 255, 255])
    pub bg_color: [u8; 3],
    /// Scale factor for output size (defaults to 4)
    pub scale: u32,
}

impl Default for QrConfig {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER_SIZE,
            fg_color: [0, 0, 0],
            bg_color: [255, 255, 255],
            scale: DEFAULT_SCALE,
        }
    }
}

impl QrConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_border(mut self, border: u32) -> Result<Self, HelperError> {
        if border > MAX_BORDER_SIZE {
            return Err(HelperError::InvalidInput(format!(
                "Border size cannot exceed {} modules",
                MAX_BORDER_SIZE
            )));
        }
        self.border = border;
        Ok(self)
    }

    pub fn with_fg_color(mut self, color: [u8; 3]) -> Self {
        self.fg_color = color;
        self
    }

    pub fn with_bg_color(mut self, color: [u8; 3]) -> Self {
        self.bg_color = color;
        self
    }

    pub fn with_scale(mut self, scale: u32) -> Result<Self, HelperError> {
        if scale == 0 || scale > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Scale must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        self.scale = scale;
        Ok(self)
    }

    /// Validates the configuration before use
    pub fn validate(&self) -> Result<(), HelperError> {
        if self.scale == 0 {
            return Err(HelperError::InvalidInput(
                "Scale must be greater than 0".to_string(),
            ));
        }
        if self.border > MAX_BORDER_SIZE {
            return Err(HelperError::InvalidInput(format!(
                "Border size cannot exceed {}",
                MAX_BORDER_SIZE
            )));
        }
        Ok(())
    }
}

/// Configuration for styled QR codes with frames and logos.
#[derive(Debug, Clone)]
pub struct FrameQrConfig<'a> {
    pub logo_path: &'a str,
    pub ecc: QrCodeEcc,
    pub upscale_factor: u32,
    pub directory_path: &'a str,
    pub file_name: Option<&'a str>,
    pub qr_color: [u8; 3],
    pub outer_frame_px: u32,
    pub inner_frame_px: u32,
    pub frame_style: FrameStyle,
}

impl<'a> Default for FrameQrConfig<'a> {
    fn default() -> Self {
        Self {
            logo_path: "",
            ecc: QrCodeEcc::High,
            upscale_factor: 8,
            directory_path: "generated",
            file_name: None,
            qr_color: [0, 0, 0],
            outer_frame_px: 0,
            inner_frame_px: 0,
            frame_style: FrameStyle::None,
        }
    }
}

impl<'a> FrameQrConfig<'a> {
    pub fn new(logo_path: &'a str) -> Result<Self, HelperError> {
        if logo_path.is_empty() {
            return Err(HelperError::InvalidInput(
                "Logo path cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            logo_path,
            ..Default::default()
        })
    }

    pub fn with_ecc(mut self, ecc: QrCodeEcc) -> Self {
        self.ecc = ecc;
        self
    }

    pub fn with_upscale(mut self, factor: u32) -> Result<Self, HelperError> {
        if factor == 0 || factor > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Upscale factor must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        self.upscale_factor = factor;
        Ok(self)
    }

    pub fn with_directory(mut self, path: &'a str) -> Self {
        self.directory_path = path;
        self
    }

    pub fn with_filename(mut self, name: &'a str) -> Self {
        self.file_name = Some(name);
        self
    }

    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.qr_color = color;
        self
    }

    pub fn with_outer_frame(mut self, size: u32) -> Self {
        self.outer_frame_px = size;
        self
    }

    pub fn with_inner_frame(mut self, size: u32) -> Self {
        self.inner_frame_px = size;
        self
    }

    pub fn with_frame_style(mut self, style: FrameStyle) -> Self {
        self.frame_style = style;
        self
    }

    /// Validates the configuration before use
    pub fn validate(&self) -> Result<(), HelperError> {
        if self.logo_path.is_empty() {
            return Err(HelperError::InvalidInput(
                "Logo path is required".to_string(),
            ));
        }
        if self.upscale_factor == 0 || self.upscale_factor > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Upscale factor must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        Ok(())
    }
}

/// Encodes a byte slice into a base64-encoded string.
///
/// Converts each group of 3 input bytes into 4 output characters from the base64 alphabet (A-Z, a-z,
/// 0-9, +, /), with padding (`=`) for inputs not divisible by 3. Optimized for minimal memory
/// allocation using precomputed capacity.
///
/// # Arguments
///
/// * `data` - A slice of bytes to encode.
///
/// # Returns
///
/// A `String` containing the base64-encoded representation.
///
/// # Example
///
/// ```rust
/// use qirust::helper::encode_base64;
///
/// let data = b"Hello";
/// let encoded = encode_base64(data);
/// assert_eq!(encoded, "SGVsbG8=");
/// ```
///
/// # Performance
///
/// Uses `String::with_capacity` to avoid reallocations and bitwise operations for encoding, making it
/// suitable for performance-critical applications.
pub fn encode_base64(data: &[u8]) -> String {
    const BASE64_ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let encoded_len = data.len().div_ceil(3) * 4;
    let mut result = String::with_capacity(encoded_len);

    let mut i = 0;
    while i + 3 <= data.len() {
        let b1 = data[i];
        let b2 = data[i + 1];
        let b3 = data[i + 2];

        result.push(BASE64_ALPHABET[(b1 >> 2) as usize] as char);
        result.push(BASE64_ALPHABET[(((b1 & 0b00000011) << 4) | (b2 >> 4)) as usize] as char);
        result.push(BASE64_ALPHABET[(((b2 & 0b00001111) << 2) | (b3 >> 6)) as usize] as char);
        result.push(BASE64_ALPHABET[(b3 & 0b00111111) as usize] as char);

        i += 3;
    }

    if data.len() - i == 1 {
        let b1 = data[i];
        result.push(BASE64_ALPHABET[(b1 >> 2) as usize] as char);
        result.push(BASE64_ALPHABET[((b1 & 0b00000011) << 4) as usize] as char);
        result.push('=');
        result.push('=');
    } else if data.len() - i == 2 {
        let b1 = data[i];
        let b2 = data[i + 1];
        result.push(BASE64_ALPHABET[(b1 >> 2) as usize] as char);
        result.push(BASE64_ALPHABET[(((b1 & 0b00000011) << 4) | (b2 >> 4)) as usize] as char);
        result.push(BASE64_ALPHABET[((b2 & 0b00001111) << 2) as usize] as char);
        result.push('=');
    }

    result
}

/// Generates an SVG string for a QR code.
///
/// Produces an SVG with a white background and black modules, using Unix newlines (`\n`). Modules are
/// grouped horizontally to reduce path elements, improving rendering performance for large QR codes.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to render.
/// * `border` - Number of border modules (must be non-negative).
///
/// # Returns
///
/// A `String` containing the SVG code.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::to_svg_string;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::Low,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// let svg = to_svg_string(&qr, 4);
/// println!("{}", svg);
/// ```
///
/// # Performance
///
/// Optimized with `String::with_capacity` for minimal reallocations and horizontal module grouping to
/// reduce SVG path complexity, making it efficient for high-version QR codes (e.g., Version 40).
pub fn to_svg_string(qr: &QrCode, border: i32) -> String {
    let qr_size = qr.size() as usize;
    let dimension = qr.size() + border * 2;
    let capacity = 200 + qr_size * qr_size * 20 + 100;
    let mut result = String::with_capacity(capacity);

    // Writing to String cannot fail, but we handle it for consistency
    let _ = writeln!(
        result,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n\
         <svg xmlns=\"http://www.w3.org/200intro/svg\" version=\"1.1\" viewBox=\"0 0 {} {}\" stroke=\"none\">\n\
         \t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n",
        dimension,
        dimension
    );

    let mut path = Vec::new();
    for y in 0..qr.size() {
        let mut x = 0;
        while x < qr.size() {
            if qr.get_module(x, y) {
                let start_x = x;
                let mut width = 1;
                while x + 1 < qr.size() && qr.get_module(x + 1, y) {
                    x += 1;
                    width += 1;
                }
                path.push(format!(
                    " M{},{}h{}v1h-{}z",
                    start_x + border,
                    y + border,
                    width,
                    width
                ));
            }
            x += 1;
        }
    }
    let _ = writeln!(
        result,
        "\t<path d=\"{}\" fill=\"#000000\"/>\n</svg>\n",
        path.join("")
    );
    result
}

/// Defines the style of the frame behind the logo in styled QR codes.
///
/// Used in functions like [frameqr_to_svg_string], [frameqr_to_image_and_save], and
/// [generate_frameqr_buffer] to specify whether the logo has a square frame, a rounded (circular)
/// frame, or no frame at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameStyle {
    /// A square frame, rendered as a white rectangle around the logo.
    Square,
    /// A rounded (circular) frame, rendered as a white circle around the logo.
    Rounded,
    /// No frame, with the logo directly overlaid on the QR code.
    None,
}

/// Configuration for SVG styled QR codes
#[derive(Debug, Clone)]
pub struct FrameQrSvgConfig<'a> {
    pub logo_path: &'a str,
    pub upscale_factor: u32,
    pub qr_color: [u8; 3],
    pub outer_frame_px: u32,
    pub inner_frame_px: u32,
    pub frame_style: FrameStyle,
}

impl<'a> Default for FrameQrSvgConfig<'a> {
    fn default() -> Self {
        Self {
            logo_path: "",
            upscale_factor: DEFAULT_UPSCALE_FACTOR,
            qr_color: [0, 0, 0],
            outer_frame_px: 0,
            inner_frame_px: DEFAULT_INNER_FRAME,
            frame_style: FrameStyle::None,
        }
    }
}

impl<'a> FrameQrSvgConfig<'a> {
    pub fn new(logo_path: &'a str) -> Result<Self, HelperError> {
        if logo_path.is_empty() {
            return Err(HelperError::InvalidInput(
                "Logo path cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            logo_path,
            ..Default::default()
        })
    }

    pub fn with_upscale(mut self, factor: u32) -> Result<Self, HelperError> {
        if factor == 0 || factor > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Upscale factor must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        self.upscale_factor = factor;
        Ok(self)
    }

    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.qr_color = color;
        self
    }

    pub fn with_outer_frame(mut self, size: u32) -> Self {
        self.outer_frame_px = size;
        self
    }

    pub fn with_inner_frame(mut self, size: u32) -> Self {
        self.inner_frame_px = size;
        self
    }

    pub fn with_frame_style(mut self, style: FrameStyle) -> Self {
        self.frame_style = style;
        self
    }
}

/// Generates an SVG string for a styled QR code with an embedded logo.
///
/// Renders a QR code with a logo embedded as a base64-encoded PNG, supporting custom colors, outer
/// frames, and square or rounded frames behind the logo. Uses horizontal module grouping for
/// efficiency and a global cache for logo base64 encoding to reduce redundant processing.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to render.
/// * `config` - Configuration for styling ([FrameQrSvgConfig]).
///
/// # Returns
///
/// A `Result` containing the SVG string or an [image::ImageError] on failure.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::{frameqr_to_svg_string, FrameQrSvgConfig, FrameStyle};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::High,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// let config = FrameQrSvgConfig::new("logo.png").unwrap()
///     .with_upscale(6).unwrap()
///     .with_color([255, 165, 0])
///     .with_outer_frame(40)
///     .with_inner_frame(10)
///     .with_frame_style(FrameStyle::Rounded);
///
/// let svg = frameqr_to_svg_string(qr, config).expect("Failed to generate SVG");
/// println!("{}", svg);
/// ```
pub fn frameqr_to_svg_string(
    qr: QrCode,
    config: FrameQrSvgConfig,
) -> Result<String, image::ImageError> {
    static LOGO_BASE64_CACHE: Mutex<Option<(String, String)>> = Mutex::new(None);

    let qr_size = qr.size() as u32;
    let upscale = config.upscale_factor;
    let outer_frame = config.outer_frame_px;
    let inner_frame = config.inner_frame_px;

    let estimated_size = 200 + qr_size * qr_size * 16 + 500 + qr_size * upscale * 4;
    let mut result = String::with_capacity(estimated_size as usize);
    let dimension = qr_size * upscale + 2 * outer_frame;

    writeln!(
        result,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n",
        dimension
    ).unwrap();

    // Render QR modules with horizontal grouping
    let mut path_buffer = Vec::with_capacity((qr_size as usize) * (qr_size as usize) * 20);
    for y in 0..qr_size {
        let mut x = 0;
        while x < qr_size {
            if qr.get_module(x as i32, y as i32) {
                let start_x = x;
                let mut width = 1;
                while x + 1 < qr_size && qr.get_module((x + 1) as i32, y as i32) {
                    x += 1;
                    width += 1;
                }
                let px = start_x * upscale + outer_frame;
                let py = y * upscale + outer_frame;
                write!(
                    &mut path_buffer,
                    "M{} {}h{}v{}h-{}z ",
                    px,
                    py,
                    width * upscale,
                    upscale,
                    width * upscale
                )
                .unwrap();
            }
            x += 1;
        }
    }

    writeln!(
        result,
        "<path d=\"{}\" fill=\"#{:02x}{:02x}{:02x}\"/>",
        std::str::from_utf8(&path_buffer).unwrap().trim(),
        config.qr_color[0],
        config.qr_color[1],
        config.qr_color[2]
    )
    .unwrap();

    // Load and encode logo
    let logo = image::open(config.logo_path)?.to_rgba8();
    let max_logo_w = (qr_size * upscale) / LOGO_SIZE_DIVISOR;
    let max_logo_h = (qr_size * upscale) / LOGO_SIZE_DIVISOR;

    let logo_resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
        image::imageops::resize(
            &logo,
            max_logo_w,
            max_logo_h,
            image::imageops::FilterType::Triangle,
        )
    } else {
        logo
    };

    let logo_base64 = {
        let cache = LOGO_BASE64_CACHE.lock().unwrap();
        if let Some((cached_path, cached_base64)) = cache.as_ref() {
            if cached_path == config.logo_path {
                cached_base64.clone()
            } else {
                let logo_buffer = {
                    let mut logo_buffer = Vec::new();
                    DynamicImage::ImageRgba8(logo_resized.clone()).write_to(
                        &mut std::io::Cursor::new(&mut logo_buffer),
                        ImageFormat::Png,
                    )?;
                    logo_buffer
                };
                encode_base64(&logo_buffer)
            }
        } else {
            let logo_buffer = {
                let mut logo_buffer = Vec::new();
                DynamicImage::ImageRgba8(logo_resized.clone()).write_to(
                    &mut std::io::Cursor::new(&mut logo_buffer),
                    ImageFormat::Png,
                )?;
                logo_buffer
            };
            encode_base64(&logo_buffer)
        }
    };

    let logo_center_x = (qr_size * upscale) / 2 + outer_frame;
    let logo_center_y = (qr_size * upscale) / 2 + outer_frame;
    let base_logo_radius = max_logo_w.min(max_logo_h) / 2;
    let logo_radius = base_logo_radius + inner_frame;

    // Apply frame style
    match config.frame_style {
        FrameStyle::Rounded => {
            writeln!(
                result,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#FFFFFF\"/>",
                logo_center_x, logo_center_y, logo_radius
            )
            .unwrap();
        }
        FrameStyle::Square => {
            writeln!(
                result,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#FFFFFF\" stroke=\"#FFFFFF\" stroke-width=\"{}\"/>",
                logo_center_x - max_logo_w / 2,
                logo_center_y - max_logo_h / 2,
                max_logo_w,
                max_logo_h,
                inner_frame * 2
            ).unwrap();
        }
        FrameStyle::None => {}
    }

    write!(
        result,
        "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\" preserveAspectRatio=\"xMidYMid meet\"/>\n</svg>\n",
        logo_center_x - max_logo_w / 2,
        logo_center_y - max_logo_h / 2,
        max_logo_w,
        max_logo_h,
        logo_base64
    ).unwrap();

    Ok(result)
}

/// Prints a QR code to the console using ASCII characters.
///
/// Uses `█` for dark modules and spaces for light modules, with a fixed 4-module border for clarity.
/// Each module is represented by two characters for better visibility.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to print.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::print_qr;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::Low,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// print_qr(&qr);
/// ```
///
/// # Performance
///
/// Minimal overhead due to simple iteration over QR modules and direct console output. Suitable for
/// quick debugging or terminal-based applications.
pub fn print_qr(qr: &QrCode) {
    let border: i32 = 4;
    for y in -border..qr.size() + border {
        for x in -border..qr.size() + border {
            let c: char = if qr.get_module(x, y) { '█' } else { ' ' };
            print!("{0}{0}", c);
        }
        println!();
    }
    println!();
}

/// Saves a QR code as a PNG image.
///
/// Renders a basic QR code with a black-and-white color scheme and a 4-module border, saving it to
/// a PNG file. The output directory is created if it does not exist.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to render.
/// * `directory_path` - Optional directory path (defaults to "generated").
/// * `filename` - Optional filename without extension (defaults to a timestamp in seconds since Unix epoch, e.g., "1716158094s").
///
/// # Returns
///
/// A `Result` indicating success or an [image::ImageError] on failure (e.g., invalid directory path).
///
/// # Example
///
/// ```rust,no_run
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::qr_to_image_and_save;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::Low,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// qr_to_image_and_save(&qr, Some("output"), Some("qr_code"))
///     .expect("Failed to save QR code");
/// ```
///
/// # Performance
///
/// Efficient for small to medium QR codes due to single-pass rendering. For large QR codes (e.g.,
/// Version 40), consider using [generate_image_buffer] for in-memory processing to avoid immediate
/// disk I/O.
pub fn qr_to_image_and_save(
    qr: &QrCode,
    directory_path: Option<&str>,
    filename: Option<&str>,
) -> Result<(), image::ImageError> {
    let border: i32 = 4;
    let size = (qr.size() as u32) + 2 * (border as u32);
    let mut img = ImageBuffer::new(size, size);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let qr_x = (x as i32) - border;
        let qr_y = (y as i32) - border;
        *pixel = if qr.get_module(qr_x, qr_y) {
            Luma([0u8]) // Black
        } else {
            Luma([255u8]) // White
        };
    }

    let directory_path = PathBuf::from(directory_path.unwrap_or("generated"));
    let filename = match filename {
        Some(name) => name.to_string(),
        None => {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            format!("{:?}", since_the_epoch)
        }
    };

    let file_path = directory_path.join(format!("{}.png", filename));

    if !Path::new(&directory_path).exists() {
        fs::create_dir_all(directory_path)?;
    }

    img.save(&Path::new(&file_path))
}

/// Configuration for saving styled QR codes with frames and logos
#[derive(Debug, Clone)]
pub struct FrameQrSaveConfig<'a> {
    pub logo_path: &'a str,
    pub upscale_factor: u32,
    pub directory_path: &'a str,
    pub file_name: Option<&'a str>,
    pub qr_color: [u8; 3],
    pub outer_frame_px: u32,
    pub inner_frame_px: u32,
    pub frame_style: FrameStyle,
}

impl<'a> Default for FrameQrSaveConfig<'a> {
    fn default() -> Self {
        Self {
            logo_path: "",
            upscale_factor: DEFAULT_UPSCALE_FACTOR,
            directory_path: "generated",
            file_name: None,
            qr_color: [0, 0, 0],
            outer_frame_px: 0,
            inner_frame_px: DEFAULT_INNER_FRAME,
            frame_style: FrameStyle::None,
        }
    }
}

impl<'a> FrameQrSaveConfig<'a> {
    pub fn new(logo_path: &'a str) -> Result<Self, HelperError> {
        if logo_path.is_empty() {
            return Err(HelperError::InvalidInput(
                "Logo path cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            logo_path,
            ..Default::default()
        })
    }

    pub fn with_upscale(mut self, factor: u32) -> Result<Self, HelperError> {
        if factor == 0 || factor > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Upscale factor must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        self.upscale_factor = factor;
        Ok(self)
    }

    pub fn with_directory(mut self, path: &'a str) -> Self {
        self.directory_path = path;
        self
    }

    pub fn with_filename(mut self, name: &'a str) -> Self {
        self.file_name = Some(name);
        self
    }

    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.qr_color = color;
        self
    }

    pub fn with_outer_frame(mut self, size: u32) -> Self {
        self.outer_frame_px = size;
        self
    }

    pub fn with_inner_frame(mut self, size: u32) -> Self {
        self.inner_frame_px = size;
        self
    }

    pub fn with_frame_style(mut self, style: FrameStyle) -> Self {
        self.frame_style = style;
        self
    }

    pub fn validate(&self) -> Result<(), HelperError> {
        if self.logo_path.is_empty() {
            return Err(HelperError::InvalidInput(
                "Logo path is required".to_string(),
            ));
        }
        if self.upscale_factor == 0 || self.upscale_factor > MAX_UPSCALE_FACTOR {
            return Err(HelperError::InvalidInput(format!(
                "Upscale factor must be between 1 and {}",
                MAX_UPSCALE_FACTOR
            )));
        }
        Ok(())
    }
}

/// Saves a styled QR code with an embedded logo as a PNG image.
///
/// Renders a QR code with a logo, custom colors, and optional square or rounded frames. The logo is
/// resized to one-third of the QR code dimensions for scannability, and a global cache is used to
/// avoid redundant resizing. The output directory is created if it does not exist.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to render.
/// * `config` - Configuration for styling and output ([FrameQrSaveConfig]).
///
/// # Returns
///
/// A `Result` indicating success or an [image::ImageError] on failure.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::{frameqr_to_image_and_save, FrameQrSaveConfig, FrameStyle};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::High,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// let config = FrameQrSaveConfig::new("logo.png").unwrap()
///     .with_upscale(6).unwrap()
///     .with_directory("output")
///     .with_filename("styled_qr")
///     .with_color([255, 165, 0])
///     .with_outer_frame(40)
///     .with_inner_frame(10)
///     .with_frame_style(FrameStyle::Rounded);
///
/// frameqr_to_image_and_save(qr, config).expect("Failed to save styled QR code");
/// ```
///
/// # Performance
///
/// Optimized with horizontal module grouping and a global `Mutex`-based cache for logo resizing,
/// reducing redundant processing. Uses `FilterType::Nearest` for fast logo resizing, suitable for
/// most applications.
///
/// # Notes
///
/// - The logo is resized to one-third of the QR code dimensions to ensure scannability.
/// - Ensure the logo file exists and is accessible before calling.
/// - For in-memory processing, consider using [generate_frameqr_buffer] to avoid immediate disk I/O.
pub fn frameqr_to_image_and_save(
    qr: QrCode,
    config: FrameQrSaveConfig,
) -> Result<(), image::ImageError> {
    // Validate config
    config.validate().map_err(|e| {
        image::ImageError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            e.to_string(),
        ))
    })?;

    let qr_size = qr.size() as u32;
    let mut qr_img = ImageBuffer::new(qr_size, qr_size);

    // Render QR code modules
    for y in 0..qr_size {
        for x in 0..qr_size {
            let color = if qr.get_module(x as i32, y as i32) {
                Rgb(config.qr_color)
            } else {
                Rgb([255, 255, 255])
            };
            qr_img.put_pixel(x, y, color);
        }
    }

    // Upscale QR code
    let mut upscaled_qr = resize(
        &DynamicImage::ImageRgb8(qr_img),
        qr_size * config.upscale_factor,
        qr_size * config.upscale_factor,
        FilterType::Nearest,
    );

    // Load and validate logo
    let full_path = env::current_dir()?.join(config.logo_path);
    if !full_path.exists() {
        return Err(image::ImageError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Logo file not found: {:?}", full_path),
        )));
    }
    let logo = image::open(&full_path)?.to_rgba8();

    // Calculate logo dimensions
    let max_logo_w = upscaled_qr.width() / LOGO_SIZE_DIVISOR;
    let max_logo_h = upscaled_qr.height() / LOGO_SIZE_DIVISOR;

    // Cache for resized logos
    static LOGO_RESIZE_CACHE: Mutex<Option<(String, u32, u32, RgbaImage)>> = Mutex::new(None);

    let logo_resized = {
        let mut cache = LOGO_RESIZE_CACHE.lock().unwrap();
        if let Some((cached_path, cached_w, cached_h, cached_logo)) = cache.as_ref() {
            if cached_path == config.logo_path && *cached_w == max_logo_w && *cached_h == max_logo_h
            {
                cached_logo.clone()
            } else {
                let resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
                    resize(&logo, max_logo_w, max_logo_h, FilterType::Nearest)
                } else {
                    logo
                };
                *cache = Some((
                    config.logo_path.to_string(),
                    max_logo_w,
                    max_logo_h,
                    resized.clone(),
                ));
                resized
            }
        } else {
            let resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
                resize(&logo, max_logo_w, max_logo_h, FilterType::Nearest)
            } else {
                logo
            };
            *cache = Some((
                config.logo_path.to_string(),
                max_logo_w,
                max_logo_h,
                resized.clone(),
            ));
            resized
        }
    };

    // Calculate logo position
    let x_offset = (upscaled_qr.width() - logo_resized.width()) / 2;
    let y_offset = (upscaled_qr.height() - logo_resized.height()) / 2;

    // Apply frame style
    match config.frame_style {
        FrameStyle::Rounded => {
            let margin = config.inner_frame_px;
            let radius = (logo_resized.width().min(logo_resized.height()) + 2 * margin) / 2;
            let mask = create_circle_mask(radius * 2, radius as i32);
            let mask_x = x_offset + logo_resized.width() / 2 - radius;
            let mask_y = y_offset + logo_resized.height() / 2 - radius;
            overlay(&mut upscaled_qr, &mask, mask_x as i64, mask_y as i64);
        }
        FrameStyle::Square => {
            let frame_margin = config.inner_frame_px;
            for y in y_offset.saturating_sub(frame_margin)
                ..(y_offset + logo_resized.height() + frame_margin).min(upscaled_qr.height())
            {
                for x in x_offset.saturating_sub(frame_margin)
                    ..(x_offset + logo_resized.width() + frame_margin).min(upscaled_qr.width())
                {
                    upscaled_qr.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                }
            }
        }
        FrameStyle::None => {}
    }

    // Overlay logo
    overlay(
        &mut upscaled_qr,
        &logo_resized,
        x_offset as i64,
        y_offset as i64,
    );

    // Prepare output path
    let filename = config.file_name.map(|s| s.to_string()).unwrap_or_else(|| {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        format!("{:?}", since_the_epoch)
    });

    let file_path = format!("{}/{}.png", config.directory_path, filename);

    // Create directory if it doesn't exist
    if !Path::new(config.directory_path).exists() {
        fs::create_dir_all(config.directory_path)?;
    }

    // Save with or without outer frame
    if config.outer_frame_px > 0 {
        let frame_px = config.outer_frame_px;
        let final_w = upscaled_qr.width() + frame_px * 2;
        let final_h = upscaled_qr.height() + frame_px * 2;
        let mut final_image =
            RgbaImage::from_pixel(final_w, final_h, image::Rgba([255, 255, 255, 255]));
        overlay(
            &mut final_image,
            &upscaled_qr,
            frame_px as i64,
            frame_px as i64,
        );
        final_image.save(&Path::new(&file_path))
    } else {
        upscaled_qr.save(&Path::new(&file_path))
    }
}

/// Generates and saves a styled QR code from text content.
///
/// A convenience wrapper that encodes text into a QR code and renders it with a logo, custom colors,
/// and optional frames. Uses the configuration from [FrameQrConfig] for flexibility.
///
/// # Arguments
///
/// * `content` - The text to encode (must not be empty).
/// * `config` - Configuration for styling and output ([FrameQrConfig]).
///
/// # Returns
///
/// * `Ok(())` - Successfully generated and saved the QR code.
/// * `Err(HelperError)` - Failed to generate or save the QR code.
///
/// # Errors
///
/// This function will return an error if:
/// * The content is empty or too long for the QR code capacity.
/// * The logo file cannot be found or opened.
/// * The output directory cannot be created.
/// * The image cannot be saved.
///
/// # Example
///
/// ```rust,no_run
/// use qirust::helper::{generate_frameqr, FrameQrConfig, FrameStyle};
/// use qirust::qrcode::QrCodeEcc;
///
/// let config = FrameQrConfig::new("logo.png").unwrap()
///     .with_ecc(QrCodeEcc::High)
///     .with_upscale(6).unwrap()
///     .with_directory("output")
///     .with_filename("styled_qr")
///     .with_color([255, 165, 0])
///     .with_outer_frame(40)
///     .with_inner_frame(10)
///     .with_frame_style(FrameStyle::Rounded);
///
/// generate_frameqr("https://example.com", config)
///     .expect("Failed to generate QR code");
/// ```
///
/// # Performance
///
/// Inherits optimizations from [frameqr_to_image_and_save], including horizontal module grouping and
/// logo resizing caching. Suitable for most use cases.
///
/// # Notes
///
/// - The logo is resized to one-third of the QR code dimensions to ensure scannability.
/// - Ensure the logo file exists and is accessible before calling.
/// - For invalid input data, the underlying [QrCode::encode_text] may return a [DataTooLong] error.
pub fn generate_frameqr(content: &str, config: FrameQrConfig) -> Result<(), HelperError> {
    // Validate input
    if content.is_empty() {
        return Err(HelperError::InvalidInput(
            "Content cannot be empty".to_string(),
        ));
    }

    // Validate config
    config.validate()?;

    // Encode QR code
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        content,
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: config.ecc,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    )?;
    std::mem::drop(tempbuffer);

    // Convert to save config
    let save_config = FrameQrSaveConfig {
        logo_path: config.logo_path,
        upscale_factor: config.upscale_factor,
        directory_path: config.directory_path,
        file_name: config.file_name,
        qr_color: config.qr_color,
        outer_frame_px: config.outer_frame_px,
        inner_frame_px: config.inner_frame_px,
        frame_style: config.frame_style,
    };

    frameqr_to_image_and_save(qr, save_config).map_err(HelperError::from)
}

/// Generates and saves a basic QR code image from text content.
///
/// Encodes the input text into a QR code with a low error correction level and saves it as a PNG
/// image with a 4-module border. The output directory is created if it does not exist.
///
/// # Arguments
///
/// * `content` - The text to encode.
/// * `directory` - Optional directory path (defaults to "generated").
/// * `filename` - Optional filename without extension (defaults to a timestamp in seconds since Unix epoch, e.g., "1716158094s").
///
/// # Returns
///
/// A `Result` indicating success or an [image::ImageError] on failure (e.g., invalid directory path).
///
/// # Example
///
/// ```rust
/// use qirust::helper::generate_image;
///
/// generate_image("Hello, World!", Some("output"), Some("qr_code"))
///     .expect("Failed to generate QR code");
/// ```
///
/// # Performance
///
/// Efficient for small to medium QR codes due to single-pass rendering. For large QR codes or
/// in-memory processing, consider using [generate_image_buffer].
///
/// # Notes
///
/// - Uses a low error correction level ([QrCodeEcc::Low]) for maximum data capacity.
/// - For invalid input data, the underlying [QrCode::encode_text] may return a [DataTooLong] error.
pub fn generate_image(
    content: &str,
    directory: Option<&str>,
    filename: Option<&str>,
) -> Result<(), HelperError> {
    // Validate input
    if content.is_empty() {
        return Err(HelperError::InvalidInput(
            "Content cannot be empty".to_string(),
        ));
    }

    let text: &str = content;
    let errcorlvl: QrCodeEcc = QrCodeEcc::Low;

    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(
        text,
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: errcorlvl,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    )?;
    std::mem::drop(tempbuffer);
    qr_to_image_and_save(&qr, directory, filename).map_err(HelperError::from)
}

/// Generates an SVG string for a QR code from text content.
///
/// Encodes the input text into a QR code with a high error correction level and renders it as an SVG
/// string with a 4-module border.
///
/// # Arguments
///
/// * `content` - The text to encode.
///
/// # Returns
///
/// A `String` containing the SVG code.
///
/// # Example
///
/// ```rust
/// use qirust::helper::generate_svg_string;
///
/// let svg = generate_svg_string("Hello, World!")
///     .expect("Failed to generate SVG");
/// println!("{}", svg);
/// ```
///
/// # Performance
///
/// Inherits optimizations from [to_svg_string], including horizontal module grouping and minimal
/// reallocations. Efficient for all QR code versions.
///
/// # Notes
///
/// - Uses a high error correction level ([QrCodeEcc::High]) for robustness.
/// - For invalid input data, the underlying [QrCode::encode_text] may return a [DataTooLong] error,
///   which is converted to a panic in this function.
pub fn generate_svg_string(content: &str) -> Result<String, HelperError> {
    // Validate input
    if content.is_empty() {
        return Err(HelperError::InvalidInput(
            "Content cannot be empty".to_string(),
        ));
    }

    let text: &str = content;
    let errcorlvl: QrCodeEcc = QrCodeEcc::High;

    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(
        text,
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: errcorlvl,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    )?;
    std::mem::drop(tempbuffer);
    Ok(to_svg_string(&qr, 4))
}

/// Mixes foreground and background colors based on a pixel value.
///
/// Blends two colors linearly based on the pixel intensity (0–255), useful for rendering smooth
/// transitions in QR code images.
///
/// # Arguments
///
/// * `pixel` - The pixel intensity (0–255).
/// * `foreground` - The foreground color value (0–255).
/// * `background` - The background color value (0–255).
///
/// # Returns
///
/// The mixed color value (0–255).
///
/// # Example
///
/// ```rust
/// use qirust::helper::mix_colors;
///
/// let mixed = mix_colors(128, 255, 0); // 50% red, 50% no red
/// assert_eq!(mixed, 128);
/// ```
///
/// # Performance
///
/// Uses integer arithmetic with minimal overhead, suitable for pixel-by-pixel processing in
/// performance-critical rendering.
pub fn mix_colors(pixel: u8, foreground: u8, background: u8) -> u8 {
    (((pixel as u16) * (foreground as u16)) / 255
        + ((255 - (pixel as u16)) * (background as u16)) / 255) as u8
}

/// Generates an in-memory image buffer for a QR code.
///
/// Encodes the input text into a QR code with a high error correction level and renders it as an
/// in-memory RGB image buffer with customizable border, colors, and scale. Uses per-pixel rendering
/// for simplicity, suitable for most use cases.
///
/// # Arguments
///
/// * `content` - The text to encode.
/// * `border` - Optional border size in modules (defaults to 4).
/// * `fg_color` - Optional foreground color as `[R, G, B]` array (defaults to black, `[0, 0, 0]`).
/// * `bg_color` - Optional background color as `[R, G, B]` array (defaults to white, `[255, 255, 255]`).
/// * `scale` - Optional scaling factor for pixel size per QR module (defaults to 4).
///
/// # Returns
///
/// A `Result` containing an [ImageBuffer] with the QR code image, or a [DataTooLong] error if the
/// content exceeds the QR code's capacity.
///
/// # Example
///
/// ```rust
/// use qirust::helper::{generate_image_buffer, QrConfig};
///
/// let config = QrConfig::new()
///     .with_border(4).unwrap()
///     .with_fg_color([255, 0, 0])
///     .with_bg_color([255, 255, 255])
///     .with_scale(6).unwrap();
///
/// let img = generate_image_buffer("Hello, World!", config)
///     .expect("Failed to generate image buffer");
/// # img.save("target/doctest_qr.png").ok();
/// ```
///
/// # Performance
///
/// Uses per-pixel rendering, which is straightforward but may be slower for high-version QR codes
/// (e.g., Version 40). For better performance, consider optimizing with horizontal module grouping.
/// Uses `FilterType::Nearest` for fast scaling and precomputed buffer sizes to minimize allocations.
///
/// # Notes
///
/// - Uses a high error correction level ([QrCodeEcc::High]) for robustness.
/// - Colors are specified as `[R, G, B]` arrays with `u8` values (0–255).
/// - The output image is in RGB format ([Rgb<u8>]) for compatibility with most image processing
///   pipelines.
/// - For styled QR codes with logos, use [generate_frameqr_buffer].
pub fn generate_image_buffer(
    content: &str,
    config: QrConfig,
) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, HelperError> {
    // Validate input
    if content.is_empty() {
        return Err(HelperError::InvalidInput(
            "Content cannot be empty".to_string(),
        ));
    }

    // Validate config
    config.validate()?;

    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        content,
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: QrCodeEcc::High,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    )?;
    std::mem::drop(tempbuffer);

    let qr_size = qr.size() as u32;
    let img_size = (qr_size + 2 * config.border) * config.scale;

    // Check if image size would be too large
    if img_size > MAX_IMAGE_DIMENSION {
        return Err(HelperError::InvalidInput(format!(
            "Generated image would be too large ({}x{}, max {}x{})",
            img_size, img_size, MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION
        )));
    }

    let mut img = ImageBuffer::from_pixel(img_size, img_size, Rgb(config.bg_color));

    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x as i32, y as i32) {
                let px = (x + config.border) * config.scale;
                let py = (y + config.border) * config.scale;
                for dy in 0..config.scale {
                    for dx in 0..config.scale {
                        img.put_pixel(px + dx, py + dy, Rgb(config.fg_color));
                    }
                }
            }
        }
    }

    Ok(img)
}

/// Generates an in-memory image buffer for a styled QR code with a logo and optional frame.
///
/// Renders a QR code with a centered logo, customizable colors, white border (in modules), and
/// optional square or rounded frame behind the logo. Uses a global cache for resized logos to
/// optimize repeated calls and horizontal module grouping for efficient rendering.
///
/// # Arguments
///
/// * `qr` - The [QrCode] to render.
/// * `logo_path` - Path to the logo image, resolved relative to the current working directory.
/// * `upscale_factor` - Optional scaling factor for output size (defaults to 8).
/// * `qr_color` - Optional RGB color for QR modules (defaults to black).
/// * `border_modules` - Optional white border (padding) around QR code, in modules (defaults to 1).
/// * `inner_frame_px` - Optional padding (in pixels) around logo frame.
/// * `frame_style` - Optional [FrameStyle] (defaults to `None`).
///
/// # Returns
///
/// An [ImageBuffer] containing the styled QR code image in RGBA format. Panics on errors such as
/// failure to load the logo or resolve the path.
///
/// # Example
///
/// ```rust,no_run
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
/// use qirust::helper::{generate_frameqr_buffer, FrameStyle};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     EncodeTextOptions {
///         ecl: QrCodeEcc::High,
///         minversion: Version::MIN,
///         maxversion: Version::MAX,
///         mask: None,
///         boostecl: true,
///     },
/// ).unwrap();
///
/// let img = generate_frameqr_buffer(
///     qr,
///     "logo.png",
///     Some(10),
///     Some([0, 0, 0]),
///     Some(4),
///     Some(10),
///     Some(FrameStyle::Rounded),
/// );
/// img.save("output/qr_styled.png").expect("Failed to save image");
/// ```
///
/// # Performance
///
/// Optimized with:
/// - Horizontal module grouping, reducing pixel operations by up to 30-50% for high-version QR codes
///   (e.g., Version 40).
/// - Global `Mutex`-based cache for resized logos, eliminating redundant resizing across calls.
/// - Uses `FilterType::Nearest` for fast logo resizing, balancing speed and quality.
///
/// # Notes
///
/// - The logo is resized to one-third of the QR code dimensions to ensure scannability.
/// - The output image is in RGBA format ([Rgba<u8>]) to support transparency in logos and frames.
/// - Ensure the logo file exists and is accessible before calling, or the function will panic.
/// - For error handling, consider using [frameqr_to_image_and_save] or [generate_frameqr].
pub fn generate_frameqr_buffer(
    qr: QrCode,
    logo_path: &str,
    upscale_factor: Option<u32>,
    qr_color: Option<[u8; 3]>,
    border_modules: Option<u32>,
    inner_frame_px: Option<u32>,
    frame_style: Option<FrameStyle>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let scale = upscale_factor.unwrap_or(8);
    let border = border_modules.unwrap_or(1);
    let qr_size = qr.size() as u32;
    let padded_size = qr_size + 2 * border;
    let mut qr_img = ImageBuffer::from_pixel(padded_size, padded_size, Rgba([255, 255, 255, 255]));
    let dark = qr_color.unwrap_or([0, 0, 0]);

    for y in 0..qr_size {
        let mut x = 0;
        while x < qr_size {
            if qr.get_module(x as i32, y as i32) {
                let start_x = x;
                let mut width = 1;
                while x + 1 < qr_size && qr.get_module((x + 1) as i32, y as i32) {
                    x += 1;
                    width += 1;
                }
                for wx in start_x..start_x + width {
                    qr_img.put_pixel(
                        wx + border,
                        y + border,
                        Rgba([dark[0], dark[1], dark[2], 255]),
                    );
                }
            }
            x += 1;
        }
    }
    let mut upscaled_qr = DynamicImage::ImageRgba8(resize(
        &DynamicImage::ImageRgba8(qr_img),
        padded_size * scale,
        padded_size * scale,
        FilterType::Nearest,
    ))
    .to_rgba8();

    let full_path = std::env::current_dir()
        .expect("Failed get root path")
        .join(logo_path);
    let logo = image::open(&full_path)
        .expect("Failed to open logo")
        .to_rgba8();
    let max_logo_w = upscaled_qr.width() / 3;
    let max_logo_h = upscaled_qr.height() / 3;
    static LOGO_RESIZE_CACHE: Mutex<Option<(String, u32, u32, RgbaImage)>> = Mutex::new(None);
    let logo_resized = {
        let mut cache = LOGO_RESIZE_CACHE.lock().unwrap();
        if let Some((cached_path, cached_w, cached_h, cached_logo)) = cache.as_ref() {
            if cached_path == logo_path && *cached_w == max_logo_w && *cached_h == max_logo_h {
                cached_logo.clone()
            } else {
                let resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
                    resize(&logo, max_logo_w, max_logo_h, FilterType::Nearest)
                } else {
                    logo
                };
                *cache = Some((
                    logo_path.to_string(),
                    max_logo_w,
                    max_logo_h,
                    resized.clone(),
                ));
                resized
            }
        } else {
            let resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
                resize(&logo, max_logo_w, max_logo_h, FilterType::Nearest)
            } else {
                logo
            };
            *cache = Some((
                logo_path.to_string(),
                max_logo_w,
                max_logo_h,
                resized.clone(),
            ));
            resized
        }
    };

    let x_offset = (upscaled_qr.width() - logo_resized.width()) / 2;
    let y_offset = (upscaled_qr.height() - logo_resized.height()) / 2;

    match frame_style.unwrap_or(FrameStyle::None) {
        FrameStyle::Rounded => {
            let margin = inner_frame_px.unwrap_or(3);
            let radius = (logo_resized.width().min(logo_resized.height()) + 2 * margin) / 2;
            let mask = create_circle_mask(radius * 2, radius as i32);
            let mask_x = x_offset + logo_resized.width() / 2 - radius;
            let mask_y = y_offset + logo_resized.height() / 2 - radius;
            overlay(&mut upscaled_qr, &mask, mask_x as i64, mask_y as i64);
        }
        FrameStyle::Square => {
            let margin = inner_frame_px.unwrap_or(3);
            let frame_img = ImageBuffer::from_pixel(
                logo_resized.width() + 2 * margin,
                logo_resized.height() + 2 * margin,
                Rgba([255, 255, 255, 255]),
            );
            replace(
                &mut upscaled_qr,
                &frame_img,
                x_offset.saturating_sub(margin) as i64,
                y_offset.saturating_sub(margin) as i64,
            );
        }
        _ => {}
    }

    overlay(
        &mut upscaled_qr,
        &logo_resized,
        x_offset as i64,
        y_offset as i64,
    );
    upscaled_qr
}

/// Converts a hexadecimal color code to an RGBA color array.
///
/// Parses a hexadecimal color string (with or without a leading `#`) into an RGBA color as a `[u8; 4]`
/// array (red, green, blue, alpha). Supports 6-character (RRGGBB) inputs with an assumed alpha of 255
/// or 8-character (RRGGBBAA) inputs for full RGBA.
///
/// # Arguments
///
/// * `hex` - A string slice containing the hexadecimal color code (e.g., "#FF0000" or "FF00007F").
///
/// # Returns
///
/// * `Ok([u8; 4])` - RGBA values `[R, G, B, A]` as `u8` (0–255).
/// * `Err(&'static str)` - An error message for invalid inputs (wrong length, non-hex characters).
///
/// # Example
///
/// ```rust
/// use qirust::helper::hex_to_rgba;
///
/// assert_eq!(hex_to_rgba("#FF0000"), Ok([255, 0, 0, 255]));
/// assert_eq!(hex_to_rgba("FF00007F"), Ok([255, 0, 0, 127]));
/// assert_eq!(
///     hex_to_rgba("FF00"),
///     Err("Hex code must be 6 (RRGGBB) or 8 (RRGGBBAA) characters")
/// );
/// ```
///
/// # Performance
///
/// Uses stack-based operations with no heap allocations, optimized with `#[inline]` for minimal
/// overhead in performance-critical rendering.
#[inline]
pub fn hex_to_rgba(hex: &str) -> Result<[u8; 4], &'static str> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 && hex.len() != 8 {
        return Err("Hex code must be 6 (RRGGBB) or 8 (RRGGBBAA) characters");
    }

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Hex code contains invalid characters");
    }

    match u32::from_str_radix(hex, 16) {
        Ok(value) => {
            let r = ((value >> (if hex.len() == 6 { 16 } else { 24 })) & 0xff) as u8;
            let g = ((value >> (if hex.len() == 6 { 8 } else { 16 })) & 0xff) as u8;
            let b = ((value >> (if hex.len() == 6 { 0 } else { 8 })) & 0xff) as u8;
            let a = if hex.len() == 8 {
                (value & 0xff) as u8
            } else {
                255
            };
            Ok([r, g, b, a])
        }
        Err(_) => Err("Invalid hex code"),
    }
}

/// Converts a hexadecimal color code to an RGB color array.
///
/// Parses a 6-character hexadecimal color string (with or without a leading `#`) into an RGB color as
/// a `[u8; 3]` array (red, green, blue).
///
/// # Arguments
///
/// * `hex` - A string slice containing the hexadecimal color code (e.g., "#FF0000" or "00FF00").
///
/// # Returns
///
/// * `Ok([u8; 3])` - RGB values `[R, G, B]` as `u8` (0–255).
/// * `Err(&'static str)` - An error message for invalid inputs (wrong length, non-hex characters).
///
/// # Example
///
/// ```rust
/// use qirust::helper::hex_to_rgb;
///
/// assert_eq!(hex_to_rgb("#FF0000"), Ok([255, 0, 0]));
/// assert_eq!(hex_to_rgb("00FF00"), Ok([0, 255, 0]));
/// assert_eq!(hex_to_rgb("FF00"), Err("Hex code must be 6 characters (RRGGBB)"));
/// ```
///
/// # Performance
///
/// Uses stack-based operations with no heap allocations, optimized with `#[inline]` for minimal
/// overhead in performance-critical rendering.
#[inline]
pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], &'static str> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Hex code must be 6 characters (RRGGBB)");
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| "Hex code contains invalid characters")?;
    Ok([
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ])
}

/// Creates a circular mask for rounded frames in styled QR codes.
///
/// Generates an [RgbaImage] with a white circular region on a transparent background, used for
/// [FrameStyle::Rounded] in functions like [generate_frameqr_buffer]. Iterates over all pixels to
/// check distance from the center, suitable for small to medium masks.
///
/// # Arguments
///
/// * `size` - The width and height of the mask image in pixels.
/// * `radius` - The radius of the circular region in pixels.
///
/// # Returns
///
/// An [RgbaImage] containing the circular mask (white circle on transparent background).
///
/// # Performance
///
/// Uses per-pixel distance checks, which may be slow for large masks. For better performance in
/// high-resolution QR codes, consider optimizing with span-based rendering.
///
/// # Notes
///
/// - The mask is centered in the image, with `size` typically set to `2 * radius` for a perfect circle.
/// - The output is in RGBA format with transparent background ([Rgba([0, 0, 0, 0])]) and white
///   foreground ([Rgba([255, 255, 255, 255])]).
fn create_circle_mask(size: u32, radius: i32) -> RgbaImage {
    let mut mask = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let center = size / 2;
    for y in 0..size {
        for x in 0..size {
            let dx = (x as i32) - (center as i32);
            let dy = (y as i32) - (center as i32);
            if dx * dx + dy * dy <= radius * radius {
                mask.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    mask
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_qr_save_config_builder() {
        let config = FrameQrSaveConfig::new("logo.png")
            .unwrap()
            .with_upscale(6)
            .unwrap()
            .with_directory("output")
            .with_filename("test")
            .with_color([255, 165, 0])
            .with_outer_frame(40)
            .with_inner_frame(10)
            .with_frame_style(FrameStyle::Rounded);

        assert_eq!(config.logo_path, "logo.png");
        assert_eq!(config.upscale_factor, 6);
        assert_eq!(config.directory_path, "output");
        assert_eq!(config.file_name, Some("test"));
        assert_eq!(config.qr_color, [255, 165, 0]);
        assert_eq!(config.outer_frame_px, 40);
        assert_eq!(config.inner_frame_px, 10);
        assert_eq!(config.frame_style, FrameStyle::Rounded);
    }

    #[test]
    fn test_frame_qr_save_config_validation() {
        // Valid config
        let valid_config = FrameQrSaveConfig::new("logo.png").unwrap();
        assert!(valid_config.validate().is_ok());

        // Invalid - empty logo path
        let invalid_path = FrameQrSaveConfig::new("");
        assert!(invalid_path.is_err());

        // Invalid upscale factor
        let config = FrameQrSaveConfig::new("logo.png").unwrap();
        let invalid_upscale = config.with_upscale(0);
        assert!(invalid_upscale.is_err());

        let config = FrameQrSaveConfig::new("logo.png").unwrap();
        let invalid_upscale = config.with_upscale(MAX_UPSCALE_FACTOR + 1);
        assert!(invalid_upscale.is_err());
    }

    #[test]
    fn test_frame_qr_svg_config_builder() {
        let config = FrameQrSvgConfig::new("logo.png")
            .unwrap()
            .with_upscale(6)
            .unwrap()
            .with_color([255, 165, 0])
            .with_outer_frame(40)
            .with_inner_frame(10)
            .with_frame_style(FrameStyle::Rounded);

        assert_eq!(config.logo_path, "logo.png");
        assert_eq!(config.upscale_factor, 6);
        assert_eq!(config.qr_color, [255, 165, 0]);
        assert_eq!(config.outer_frame_px, 40);
        assert_eq!(config.inner_frame_px, 10);
        assert_eq!(config.frame_style, FrameStyle::Rounded);
    }
}
