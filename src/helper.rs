/// Utilities for rendering QR codes.
///
/// This module provides functions to render QR codes as console output, PNG images, or SVGs, with
/// options for styling (e.g., logo embedding, custom colors, frames).
use crate::qrcode::{ DataTooLong, QrCode, QrCodeEcc, Version };
use image::{
    imageops::{ overlay, resize, FilterType },
    DynamicImage,
    ImageBuffer,
    ImageFormat,
    Luma,
    Rgb,
    Rgba,
    RgbaImage,
};
use std::{
    env,
    fmt::Write,
    fs,
    path::{ Path, PathBuf },
    sync::Mutex,
    time::{ SystemTime, UNIX_EPOCH },
};

/// Encodes a byte slice into a base64-encoded string.
///
/// This function implements standard base64 encoding, converting each group of 3 input bytes into
/// 4 output characters from the base64 alphabet (A-Z, a-z, 0-9, +, /). If the input length is not
/// a multiple of 3, padding with '=' characters is added as needed.
///
/// # Arguments
///
/// * `data` - A slice of bytes to encode.
///
/// # Returns
///
/// A `String` containing the base64-encoded representation of the input data.
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
pub fn encode_base64(data: &[u8]) -> String {
    const BASE64_ALPHABET: &[
        u8;
        64
    ] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);
        let c1 = (b1 >> 2) & 0x3f;
        let c2 = ((b1 & 0x03) << 4) | ((b2 >> 4) & 0x0f);
        let c3 = if chunk.len() > 1 { ((b2 & 0x0f) << 2) | ((b3 >> 6) & 0x03) } else { 64 };
        let c4 = if chunk.len() > 2 { b3 & 0x3f } else { 64 };
        result.push(BASE64_ALPHABET[c1 as usize] as char);
        result.push(BASE64_ALPHABET[c2 as usize] as char);
        result.push(if c3 == 64 { '=' } else { BASE64_ALPHABET[c3 as usize] as char });
        result.push(if c4 == 64 { '=' } else { BASE64_ALPHABET[c4 as usize] as char });
    }
    result
}

/// Generates an SVG string for a QR code.
///
/// The SVG uses Unix newlines (`\n`) and includes a white background with black modules.
///
/// # Arguments
///
/// * `qr` - The QR code to render.
/// * `border` - Number of border modules (must be non-negative).
///
/// # Returns
///
/// A string containing the SVG code.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::to_svg_string;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::Low,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// let svg = to_svg_string(&qr, 4);
/// println!("{}", svg);
/// ```
pub fn to_svg_string(qr: &QrCode, border: i32) -> String {
    let qr_size = qr.size() as usize;
    let capacity = 200 + qr_size * qr_size * 20 + 100;
    let mut result = String::with_capacity(capacity);
    let dimension = qr.size().checked_add(border.checked_mul(2).unwrap()).unwrap();
    writeln!(
        result,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n",
        dimension
    ).unwrap();
    let mut path = String::new();
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            if qr.get_module(x, y) {
                write!(path, " M{},{}h1v1h-1z", x + border, y + border).unwrap();
            }
        }
    }
    writeln!(result, "\t<path d=\"{}\" fill=\"#000000\"/>\n</svg>\n", path.trim_start()).unwrap();
    result
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameStyle {
    Square,
    Rounded,
    None,
}

/// Generates an SVG string for a styled QR code with an embedded logo.
///
/// Supports custom colors, outer frames, and square or rounded frames behind the logo.
/// The logo is embedded as a base64-encoded PNG image using a custom encoding function.
/// Panics on errors such as failure to load the logo or encode the image.
/// Uses CatmullRom filter for sharper logo resizing and aligns logo to integer coordinates.
///
/// # Arguments
///
/// * `qr` - The QR code to render.
/// * `logo_path` - Path to the logo image, resolved relative to the current working directory.
/// * `upscale_factor` - Optional scaling factor (defaults to 8).
/// * `qr_color` - Optional RGB color for dark modules (defaults to black).
/// * `outer_frame_px` - Optional white frame size in pixels.
/// * `inner_frame_px` - Optional inner frame size in pixels.
/// * `frame_style` - Optional frame style FrameStyle (defaults to `None`).
///
/// # Returns
///
/// A `Result` containing the SVG string or an [`image::ImageError`] on failure.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::{frameqr_to_svg_string, FrameStyle::Rounded};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::High,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// match frameqr_to_svg_string(
///     qr,
///     "src/logo.png",
///     Some(6),
///     Some([255, 165, 0]),
///     Some(40),
///     Some(10),
///     Some(Rounded),
/// ) {
///     Ok(svg) => println!("{}", svg),
///     Err(e) => eprintln!("Error generating SVG: {}", e),
/// }
/// ```
pub fn frameqr_to_svg_string(
    qr: QrCode,
    logo_path: &str,
    upscale_factor: Option<u32>,
    qr_color: Option<[u8; 3]>,
    outer_frame_px: Option<u32>,
    inner_frame_px: Option<u32>,
    frame_style: Option<FrameStyle>
) -> Result<String, image::ImageError> {
    static LOGO_BASE64_CACHE: Mutex<Option<(String, String)>> = Mutex::new(None);
    let qr_size = qr.size() as u32;
    let upscale = upscale_factor.unwrap_or(8);
    let outer_frame = outer_frame_px.unwrap_or(0);
    let inner_frame = inner_frame_px.unwrap_or(0);
    let estimated_size = 200 + qr_size * qr_size * 16 + 500 + qr_size * upscale * 4;
    let mut result = String::with_capacity(estimated_size as usize);
    let dimension = qr_size * upscale + 2 * outer_frame;
    writeln!(
        result,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n",
        dimension
    ).unwrap();
    let qr_color = qr_color.unwrap_or([0, 0, 0]);
    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x as i32, y as i32) {
                let px = x * upscale + outer_frame;
                let py = y * upscale + outer_frame;
                write!(
                    result,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#{:02x}{:02x}{:02x}\"/>\n",
                    px,
                    py,
                    upscale,
                    upscale,
                    qr_color[0],
                    qr_color[1],
                    qr_color[2]
                ).unwrap();
            }
        }
    }
    let logo = image::open(logo_path)?.to_rgba8();
    let max_logo_w = (qr_size * upscale) / 3;
    let max_logo_h = (qr_size * upscale) / 3;
    let logo_resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
        image::imageops::resize(
            &logo,
            max_logo_w,
            max_logo_h,
            image::imageops::FilterType::Triangle
        )
    } else {
        logo
    };
    let logo_base64 = {
        let mut cache = LOGO_BASE64_CACHE.lock().unwrap();
        if let Some((cached_path, cached_base64)) = cache.as_ref() {
            if cached_path == logo_path {
                cached_base64.clone()
            } else {
                let mut logo_buffer = Vec::new();
                DynamicImage::ImageRgba8(logo_resized.clone()).write_to(
                    &mut std::io::Cursor::new(&mut logo_buffer),
                    ImageFormat::Png
                )?;
                let base64 = encode_base64(&logo_buffer);
                *cache = Some((logo_path.to_string(), base64.clone()));
                base64
            }
        } else {
            let mut logo_buffer = Vec::new();
            DynamicImage::ImageRgba8(logo_resized.clone()).write_to(
                &mut std::io::Cursor::new(&mut logo_buffer),
                ImageFormat::Png
            )?;
            let base64 = encode_base64(&logo_buffer);
            *cache = Some((logo_path.to_string(), base64.clone()));
            base64
        }
    };
    let logo_center_x = (qr_size * upscale) / 2 + outer_frame;
    let logo_center_y = (qr_size * upscale) / 2 + outer_frame;
    let base_logo_radius = max_logo_w.min(max_logo_h) / 2;
    let logo_radius = base_logo_radius + inner_frame;
    match frame_style.unwrap_or(FrameStyle::None) {
        FrameStyle::Rounded => {
            write!(
                result,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#FFFFFF\"/>\n",
                logo_center_x,
                logo_center_y,
                logo_radius
            ).unwrap();
        }
        FrameStyle::Square => {
            write!(
                result,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#FFFFFF\" stroke=\"#FFFFFF\" stroke-width=\"{}\"/>\n",
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
/// Uses `█` for dark modules and spaces for light modules, with a 4-module border.
///
/// # Arguments
///
/// * `qr` - The QR code to print.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::print_qr;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::Low,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// print_qr(&qr);
/// ```
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
/// # Arguments
///
/// * `qr` - The QR code to render.
/// * `directory_path` - Optional directory path (defaults to "generated").
/// * `filename` - Optional filename (defaults to a timestamp in seconds since the Unix epoch, e.g., "1716158094s").
///
/// # Returns
///
/// A `Result` indicating success or an [`image::ImageError`] on failure.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::qr_to_image_and_save;
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "Hello, World!",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::Low,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// match qr_to_image_and_save(&qr, Some("output"), Some("qr_code")) {
///     Ok(()) => println!("QR code saved successfully"),
///     Err(e) => eprintln!("Error saving QR code: {}", e),
/// }
/// ```
pub fn qr_to_image_and_save(
    qr: &QrCode,
    directory_path: Option<&str>,
    filename: Option<&str>
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
            let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
            format!("{:?}", since_the_epoch)
        }
    };

    let file_path = directory_path.join(format!("{}.png", filename));

    // Check if the directory exists, create it if it doesn't
    if !Path::new(&directory_path).exists() {
        fs::create_dir_all(directory_path)?;
    }

    img.save(&Path::new(&file_path))
}

/// Saves a styled QR code with an embedded logo as a PNG image.
///
/// Supports custom colors, outer frames, and square or rounded frames behind the logo.
///
/// # Arguments
///
/// * `qr` - The QR code to render.
/// * `logo_path` - Path to the logo image, resolved relative to the current working directory.
/// * `upscale_factor` - Optional scaling factor (defaults to 8).
/// * `directory_path` - Optional directory path (defaults to "generated").
/// * `file_name` - Optional filename (defaults to a timestamp in seconds since the Unix epoch, e.g., "1716158094s").
/// * `qr_color` - Optional RGB color for dark modules (defaults to black).
/// * `outer_frame_px` - Optional white frame size in pixels.
/// * `inner_frame_px` - Optional inner frame size in pixels.
/// * `frame_style` - Optional frame style FrameStyle (defaults to `None`).
///
/// # Returns
///
/// A `Result` indicating success or an [`image::ImageError`] on failure.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::{frameqr_to_image_and_save, FrameStyle::Rounded};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::High,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// match frameqr_to_image_and_save(
///     qr,
///     "src/logo.png",
///     Some(6),
///     Some("output"),
///     Some("styled_qr"),
///     Some([255, 165, 0]),
///     Some(40),
///     Some(10),
///     Some(Rounded),
/// ) {
///     Ok(()) => println!("Styled QR code saved successfully"),
///     Err(e) => eprintln!("Error saving styled QR code: {}", e),
/// }
/// ```
pub fn frameqr_to_image_and_save(
    qr: QrCode,
    logo_path: &str,
    upscale_factor: Option<u32>,
    directory_path: Option<&str>,
    file_name: Option<&str>,
    qr_color: Option<[u8; 3]>,
    outer_frame_px: Option<u32>,
    inner_frame_px: Option<u32>,
    frame_style: Option<FrameStyle>
) -> Result<(), image::ImageError> {
    let qr_size = qr.size() as u32;
    let mut qr_img = ImageBuffer::new(qr_size, qr_size);

    // Draw QR with optional color
    for y in 0..qr_size {
        for x in 0..qr_size {
            let color = if qr.get_module(x as i32, y as i32) {
                Rgb(qr_color.unwrap_or([0, 0, 0]))
            } else {
                Rgb([255, 255, 255])
            };
            qr_img.put_pixel(x, y, color);
        }
    }

    let upscale = upscale_factor.unwrap_or(8);
    let mut upscaled_qr = resize(
        &DynamicImage::ImageRgb8(qr_img),
        qr_size * upscale,
        qr_size * upscale,
        FilterType::Nearest
    );

    let full_path = env::current_dir()?.join(logo_path);
    if !full_path.exists() {
        return Err(
            image::ImageError::IoError(
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Logo file not found: {:?}", full_path)
                )
            )
        );
    }
    let logo = image::open(&full_path)?.to_rgba8();
    let max_logo_w = upscaled_qr.width() / 3;
    let max_logo_h = upscaled_qr.height() / 3;
    let logo_resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
        resize(&logo, max_logo_w, max_logo_h, FilterType::Lanczos3)
    } else {
        logo
    };

    let x_offset = (upscaled_qr.width() - logo_resized.width()) / 2;
    let y_offset = (upscaled_qr.height() - logo_resized.height()) / 2;

    // Apply frame style if any
    match frame_style.unwrap_or(FrameStyle::None) {
        FrameStyle::Rounded => {
            let frame_margin = inner_frame_px.unwrap_or(3);
            let center_x = x_offset + logo_resized.width() / 2;
            let center_y = y_offset + logo_resized.height() / 2;
            let radius = ((logo_resized.width() + frame_margin * 2).min(
                logo_resized.height() + frame_margin * 2
            ) / 2) as f64;
            for y in y_offset.saturating_sub(frame_margin)..(
                y_offset +
                logo_resized.height() +
                frame_margin
            ).min(upscaled_qr.height()) {
                for x in x_offset.saturating_sub(frame_margin)..(
                    x_offset +
                    logo_resized.width() +
                    frame_margin
                ).min(upscaled_qr.width()) {
                    let dx = (x as i64) - (center_x as i64);
                    let dy = (y as i64) - (center_y as i64);
                    if ((dx * dx + dy * dy) as f64).sqrt() <= radius {
                        upscaled_qr.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
        FrameStyle::Square => {
            let frame_margin = inner_frame_px.unwrap_or(3);
            for y in y_offset.saturating_sub(frame_margin)..(
                y_offset +
                logo_resized.height() +
                frame_margin
            ).min(upscaled_qr.height()) {
                for x in x_offset.saturating_sub(frame_margin)..(
                    x_offset +
                    logo_resized.width() +
                    frame_margin
                ).min(upscaled_qr.width()) {
                    upscaled_qr.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                }
            }
        }
        FrameStyle::None => {}
    }

    overlay(&mut upscaled_qr, &logo_resized, x_offset as i64, y_offset as i64);

    let directory_path = directory_path.unwrap_or("generated");
    let filename = file_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
            format!("{:?}", since_the_epoch)
        });

    let file_path = format!("{}/{}.png", directory_path, filename);

    // Ensure directory exists
    if !Path::new(directory_path).exists() {
        fs::create_dir_all(directory_path)?;
    }

    // Add outer frame if requested
    if let Some(frame_px) = outer_frame_px {
        let final_w = upscaled_qr.width() + frame_px * 2;
        let final_h = upscaled_qr.height() + frame_px * 2;
        let mut final_image = RgbaImage::from_pixel(
            final_w,
            final_h,
            image::Rgba([255, 255, 255, 255])
        );
        overlay(&mut final_image, &upscaled_qr, frame_px as i64, frame_px as i64);
        final_image.save(&Path::new(&file_path))
    } else {
        upscaled_qr.save(&Path::new(&file_path))
    }
}

/// Generates and saves a styled QR code from text content.
///
/// A convenience wrapper around [`frameqr_to_image_and_save`].
///
/// # Arguments
///
/// * `content` - The text to encode.
/// * `logo_path` - Path to the logo image, resolved relative to the current working directory.
/// * `ecc` - Optional error correction level (defaults to `High`).
/// * `upscale_factor` - Optional scaling factor (defaults to 4).
/// * `directory_path` - Optional directory path (defaults to "generated").
/// * `file_name` - Optional filename (defaults to a timestamp in seconds since the Unix epoch, e.g., "1716158094s").
/// * `qr_color` - Optional RGB color for dark modules (defaults to black).
/// * `outer_frame_px` - Optional white frame size in pixels.
/// * `inner_frame_px` - Optional inner frame size in pixels.
/// * `frame_style` - Optional frame style FrameStyle (defaults to `None`).
///
/// # Example
///
/// ```rust
/// use qirust::{helper::{generate_frameqr, FrameStyle::Rounded}, qrcode::QrCodeEcc};
///
/// match generate_frameqr(
///     "https://example.com",
///     "src/logo.png",
///     Some(QrCodeEcc::High),
///     Some(6),
///     Some("output"),
///     Some("styled_qr"),
///     Some([255, 165, 0]),
///     Some(40),
///     Some(10),
///     Some(Rounded),
/// ) {
///     Ok(()) => println!("Styled QR code generated successfully"),
///     Err(e) => eprintln!("Error generating styled QR code: {}", e),
/// }
/// ```
pub fn generate_frameqr(
    content: &str,
    logo_path: &str,
    ecc: Option<QrCodeEcc>,
    upscale_factor: Option<u32>,
    directory_path: Option<&str>,
    file_name: Option<&str>,
    qr_color: Option<[u8; 3]>,
    outer_frame_px: Option<u32>,
    inner_frame_px: Option<u32>,
    frame_style: Option<FrameStyle>
) -> Result<(), image::ImageError> {
    let errcorlvl = ecc.unwrap_or(QrCodeEcc::High);
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        content,
        &mut tempbuffer,
        &mut outbuffer,
        errcorlvl,
        Version::MIN,
        Version::MAX,
        None,
        true
    ).unwrap();
    std::mem::drop(tempbuffer);
    frameqr_to_image_and_save(
        qr,
        logo_path,
        upscale_factor,
        directory_path,
        file_name,
        qr_color,
        outer_frame_px,
        inner_frame_px,
        frame_style
    )
}

/// Generates and saves a basic QR code image from text content.
///
/// # Arguments
///
/// * `content` - The text to encode.
/// * `directory` - Optional directory path (defaults to "generated").
/// * `filename` - Optional filename (defaults to a timestamp in seconds since the Unix epoch, e.g., "1716158094s").
///
/// # Example
///
/// ```rust
/// use qirust::helper::generate_image;
///
/// match generate_image("Hello, World!", Some("output"), Some("qr_code")) {
///     Ok(()) => println!("QR code generated successfully"),
///     Err(e) => eprintln!("Error generating QR code: {}", e),
/// }
/// ```
pub fn generate_image(
    content: &str,
    directory: Option<&str>,
    filename: Option<&str>
) -> Result<(), image::ImageError> {
    let text: &str = content; // User-supplied Unicode text
    let errcorlvl: QrCodeEcc = QrCodeEcc::Low; // Error correction level

    // Make and print the QR Code symbol
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(
        text,
        &mut tempbuffer,
        &mut outbuffer,
        errcorlvl,
        Version::MIN,
        Version::MAX,
        None,
        true
    ).unwrap();
    std::mem::drop(tempbuffer); // Optional, because tempbuffer is only needed during encode_text()
    qr_to_image_and_save(&qr, directory, filename)
}

/// Generates an SVG string for a QR code from text content.
///
/// # Arguments
///
/// * `content` - The text to encode.
///
/// # Returns
///
/// A string containing the SVG code.
///
/// # Example
///
/// ```rust
/// use qirust::helper::generate_svg_string;
///
/// let svg = generate_svg_string("Hello, World!");
/// println!("{}", svg);
/// ```
pub fn generate_svg_string(content: &str) -> String {
    let text: &str = content; // User-supplied Unicode text
    let errcorlvl: QrCodeEcc = QrCodeEcc::High; // Error correction level

    // Make and print the QR Code symbol
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(
        text,
        &mut tempbuffer,
        &mut outbuffer,
        errcorlvl,
        Version::MIN,
        Version::MAX,
        None,
        true
    ).unwrap();
    std::mem::drop(tempbuffer); // Optional, because tempbuffer is only needed during encode_text()
    to_svg_string(&qr, 4)
}

/// Mixes foreground and background colors based on a pixel value.
///
/// # Arguments
///
/// * `pixel` - The pixel value (0–255).
/// * `foreground` - The foreground color value.
/// * `background` - The background color value.
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
/// let mixed = mix_colors(128, 255, 0); // Mixes full red with no red
/// println!("{}", mixed);
/// ```
pub fn mix_colors(pixel: u8, foreground: u8, background: u8) -> u8 {
    (((pixel as u16) * (foreground as u16)) / 255 +
        ((255 - (pixel as u16)) * (background as u16)) / 255) as u8
}

/// Generates an in-memory image buffer for a QR code.
///
/// # Arguments
///
/// * `content` - The text to encode.
/// * `border` - Optional border size in modules (defaults to 4).
/// * `fg_color` - Optional foreground color (defaults to black).
/// * `bg_color` - Optional background color (defaults to white).
/// * `scale` - Optional scaling factor for pixel size per QR module (defaults to 4).
///
/// # Returns
///
/// A `Result` containing an [`ImageBuffer`] with the QR code image, or a [`DataTooLong`] error if the content is too large.
///
/// # Example
///
/// ```rust
/// use qirust::helper::generate_image_buffer;
///
/// match generate_image_buffer("Hello, World!", None, None, None, None) {
///     Ok(img) => println!("Image buffer generated with dimensions: {:?}", img.dimensions()),
///     Err(e) => eprintln!("Error generating image buffer: {:?}", e),
/// }
/// ```
pub fn generate_image_buffer(
    content: &str,
    border: Option<u32>,
    fg_color: Option<Rgb<u8>>,
    bg_color: Option<Rgb<u8>>,
    scale: Option<u32>
) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, DataTooLong> {
    let border = border.unwrap_or(4);
    let foreground_color = fg_color.unwrap_or(Rgb([0, 0, 0]));
    let background_color = bg_color.unwrap_or(Rgb([255, 255, 255]));
    let errcorlvl = QrCodeEcc::High;
    let scale = scale.unwrap_or(4);

    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        content,
        &mut tempbuffer,
        &mut outbuffer,
        errcorlvl,
        Version::MIN,
        Version::MAX,
        None,
        true
    )?;
    std::mem::drop(tempbuffer);

    let qr_size = qr.size() as u32;
    let img_size = (qr_size + 2 * border) * scale;
    let mut img = ImageBuffer::from_pixel(img_size, img_size, background_color);

    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x as i32, y as i32) {
                let px = (x + border) * scale;
                let py = (y + border) * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        img.put_pixel(px + dx, py + dy, foreground_color);
                    }
                }
            }
        }
    }

    Ok(img)
}

/// Generates an in-memory image buffer for a styled QR code with logo and optional frame.
///
/// The QR code is rendered with a configurable color, white border (in modules),
/// and optional square or rounded frame behind the centered logo.
///
/// # Arguments
///
/// * `qr` - The QR code to render.
/// * `logo_path` - Path to the logo image, resolved relative to the current working directory.
/// * `upscale_factor` - Optional scale factor for output size (defaults to 8).
/// * `qr_color` - Optional RGB color for QR modules (defaults to black).
/// * `border_modules` - White border (padding) around QR code, in modules (defaults to 1).
/// * `inner_frame_px` - Optional padding (in pixels) around logo frame.
/// * `frame_style` - Optional frame style FrameStyle (defaults to `None`).
///
/// # Returns
///
/// An `ImageBuffer<Rgba<u8>, Vec<u8>>` containing the styled QR code image with logo.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
/// use qirust::helper::{generate_frameqr_buffer, FrameStyle::Rounded};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
///
/// let qr = QrCode::encode_text(
///     "https://example.com",
///     &mut tempbuffer,
///     &mut outbuffer,
///     QrCodeEcc::High,
///     Version::MIN,
///     Version::MAX,
///     None,
///     true,
/// ).unwrap();
///
/// let image = generate_frameqr_buffer(
///     qr,
///     "src/logo.png",
///     Some(10),             // upscale factor
///     Some([0, 0, 0]),      // QR color (black)
///     Some(4),              // border in modules
///     Some(10),             // frame margin in pixels
///     Some(Rounded)         // frame style
/// );
///
/// match image.save("output/qr_styled.png") {
///     Ok(()) => println!("Styled QR code saved successfully"),
///     Err(e) => eprintln!("Error saving styled QR code: {}", e),
/// }
/// ```
pub fn generate_frameqr_buffer(
    qr: QrCode,
    logo_path: &str,
    upscale_factor: Option<u32>,
    qr_color: Option<[u8; 3]>,
    border_modules: Option<u32>,
    inner_frame_px: Option<u32>,
    frame_style: Option<FrameStyle>
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let scale = upscale_factor.unwrap_or(8);
    let border = border_modules.unwrap_or(1);
    let qr_size = qr.size() as u32;
    let padded_size = qr_size + 2 * border;

    // Create image buff with white background
    let mut qr_img = ImageBuffer::from_pixel(padded_size, padded_size, Rgba([255, 255, 255, 255]));
    let dark = qr_color.unwrap_or([0, 0, 0]);

    // Draw QR to center (offset with border)
    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x as i32, y as i32) {
                qr_img.put_pixel(x + border, y + border, Rgba([dark[0], dark[1], dark[2], 255]));
            }
        }
    }

    // Upscale all
    let mut upscaled_qr = DynamicImage::ImageRgba8(
        resize(
            &DynamicImage::ImageRgba8(qr_img),
            padded_size * scale,
            padded_size * scale,
            FilterType::Nearest
        )
    ).to_rgba8();

    // Load and resize logo
    let full_path = std::env::current_dir().unwrap().join(logo_path);
    let logo = image::open(&full_path).expect("Failed to open logo").to_rgba8();
    let max_logo_w = upscaled_qr.width() / 3;
    let max_logo_h = upscaled_qr.height() / 3;
    let logo_resized = if logo.width() > max_logo_w || logo.height() > max_logo_h {
        resize(&DynamicImage::ImageRgba8(logo), max_logo_w, max_logo_h, FilterType::Lanczos3)
    } else {
        logo
    };

    let x_offset = (upscaled_qr.width() - logo_resized.width()) / 2;
    let y_offset = (upscaled_qr.height() - logo_resized.height()) / 2;

    // Frame style for logo
    match frame_style.unwrap_or(FrameStyle::None) {
        FrameStyle::Rounded => {
            let margin = inner_frame_px.unwrap_or(3);
            let radius = ((logo_resized.width().min(logo_resized.height()) + 2 * margin) /
                2) as i32;
            let size = (radius * 2) as u32;
            let mut mask = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));

            for y in 0..size {
                for x in 0..size {
                    let dx = (x as i32) - radius;
                    let dy = (y as i32) - radius;
                    if dx * dx + dy * dy <= radius * radius {
                        mask.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                    }
                }
            }

            let mask_x = x_offset + logo_resized.width() / 2 - (radius as u32);
            let mask_y = y_offset + logo_resized.height() / 2 - (radius as u32);

            for y in 0..size {
                for x in 0..size {
                    if mask.get_pixel(x, y)[3] != 0 {
                        let px = mask_x + x;
                        let py = mask_y + y;
                        if px < upscaled_qr.width() && py < upscaled_qr.height() {
                            upscaled_qr.put_pixel(px, py, Rgba([255, 255, 255, 255]));
                        }
                    }
                }
            }
        }
        FrameStyle::Square => {
            let margin = inner_frame_px.unwrap_or(3);
            for y in y_offset.saturating_sub(margin)..(
                y_offset +
                logo_resized.height() +
                margin
            ).min(upscaled_qr.height()) {
                for x in x_offset.saturating_sub(margin)..(
                    x_offset +
                    logo_resized.width() +
                    margin
                ).min(upscaled_qr.width()) {
                    upscaled_qr.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                }
            }
        }
        _ => {}
    }

    // overlay logo
    overlay(&mut upscaled_qr, &logo_resized, x_offset as i64, y_offset as i64);
    upscaled_qr
}

/// Converts a hexadecimal color code to an RGBA color array.
///
/// This function takes a hexadecimal color string (with or without a leading `#`) and converts it
/// into an RGBA color represented as a `[u8; 4]` array, where the elements correspond to red (R),
/// green (G), blue (B), and alpha (A) channels. The input must be either 6 characters (RRGGBB) for
/// RGB with an assumed alpha of 255, or 8 characters (RRGGBBAA) for full RGBA.
///
/// # Arguments
///
/// * `hex` - A string slice (`&str`) containing the hexadecimal color code. It can start with an
///   optional `#` (e.g., "#FF0000" or "FF0000"). The code must be 6 or 8 characters long, excluding
///   the `#`, and contain only valid hexadecimal digits (0-9, A-F, a-f).
///
/// # Returns
///
/// * `Ok([u8; 4])` - An array containing the RGBA values `[R, G, B, A]`, where each value is a `u8`
///   (0–255). If the input is 6 characters, the alpha value is set to 255 (fully opaque).
/// * `Err(&'static str)` - An error message if the input is invalid. Possible errors include:
///   - Input length is not 6 or 8 characters (excluding `#`).
///   - Input contains non-hexadecimal characters.
///   - Input cannot be parsed as a valid hexadecimal number.
///
/// # Examples
///
/// ```rust
/// use qirust::helper::hex_to_rgba;
///
/// // Convert RGB hex code
/// assert_eq!(hex_to_rgba("#FF0000"), Ok([255, 0, 0, 255]));
/// assert_eq!(hex_to_rgba("00FF00"), Ok([0, 255, 0, 255]));
///
/// // Convert RGBA hex code
/// assert_eq!(hex_to_rgba("#FF00007F"), Ok([255, 0, 0, 127]));
///
/// // Invalid inputs
/// assert_eq!(hex_to_rgba("FF00"), Err("Hex code must be 6 (RRGGBB) or 8 (RRGGBBAA) characters"));
/// assert_eq!(hex_to_rgba("GG0000"), Err("Hex code contains invalid characters"));
/// ```
///
/// # Notes
///
/// - The function is marked `#[inline]` for performance, ensuring minimal overhead when called.
/// - The input is case-insensitive (e.g., "FF0000" and "ff0000" are equivalent).
/// - Leading `#` is optional and automatically removed.
/// - For 6-character inputs, the alpha channel defaults to 255 (fully opaque).
///
/// # Performance
///
/// This function is highly optimized, using stack-based operations with no heap allocations. It
/// performs lightweight string parsing and bitwise operations, making it suitable for performance-
/// critical applications.
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
            let a = if hex.len() == 8 { (value & 0xff) as u8 } else { 255 };
            Ok([r, g, b, a])
        }
        Err(_) => Err("Invalid hex code"),
    }
}

/// Converts a hexadecimal color code to an RGB color array.
///
/// This function takes a hexadecimal color string (with or without a leading `#`) and converts it
/// into an RGB color represented as a `[u8; 3]` array, where the elements correspond to red (R),
/// green (G), and blue (B) channels. The input must be exactly 6 characters (RRGGBB).
///
/// # Arguments
///
/// * `hex` - A string slice (`&str`) containing the hexadecimal color code. It can start with an
///   optional `#` (e.g., "#FF0000" or "FF0000"). The code must be 6 characters long, excluding
///   the `#`, and contain only valid hexadecimal digits (0-9, A-F, a-f).
///
/// # Returns
///
/// * `Ok([u8; 3])` - An array containing the RGB values `[R, G, B]`, where each value is a `u8`
///   (0–255).
/// * `Err(&'static str)` - An error message if the input is invalid. Possible errors include:
///   - Input length is not 6 characters (excluding `#`).
///   - Input contains non-hexadecimal characters.
///   - Input cannot be parsed as a valid hexadecimal number.
///
/// # Examples
///
/// ```rust
/// use qirust::helper::hex_to_rgb;
///
/// // Convert RGB hex code
/// assert_eq!(hex_to_rgb("#FF0000"), Ok([255, 0, 0]));
/// assert_eq!(hex_to_rgb("00FF00"), Ok([0, 255, 0]));
///
/// // Invalid inputs
/// assert_eq!(hex_to_rgb("FF00"), Err("Hex code must be 6 characters (RRGGBB)"));
/// assert_eq!(hex_to_rgb("GG0000"), Err("Hex code contains invalid characters"));
/// assert_eq!(hex_to_rgb("FF00007F"), Err("Hex code must be 6 characters (RRGGBB)"));
/// ```
///
/// # Notes
///
/// - The function is marked `#[inline]` for performance, ensuring minimal overhead when called.
/// - The input is case-insensitive (e.g., "FF0000" and "ff0000" are equivalent).
/// - Leading `#` is optional and automatically removed.
/// - Unlike `hex_to_rgba`, this function does not support alpha channels and requires exactly
///   6 characters.
///
/// # Performance
///
/// This function is highly optimized, using stack-based operations with no heap allocations. It
/// performs lightweight string parsing and bitwise operations, making it suitable for performance-
/// critical applications.
#[inline]
pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], &'static str> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Hex code must be 6 characters (RRGGBB)");
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| "Hex code contains invalid characters")?;
    Ok([((value >> 16) & 0xff) as u8, ((value >> 8) & 0xff) as u8, (value & 0xff) as u8])
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_svg_string() {
        let errcorlvl: QrCodeEcc = QrCodeEcc::Low;
        let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
        let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
        let qr = QrCode::encode_text(
            "HELLO WORLD",
            &mut tempbuffer,
            &mut outbuffer,
            errcorlvl,
            Version::MIN,
            Version::MAX,
            None,
            true
        ).unwrap();
        let svg = to_svg_string(&qr, 4);

        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn test_generate_image_buffer() {
        let content = "Hello, world!";
        let border = 4;
        let scale = 4;

        let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
        let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];

        let qr = QrCode::encode_text(
            content,
            &mut tempbuffer,
            &mut outbuffer,
            QrCodeEcc::High,
            Version::MIN,
            Version::MAX,
            None,
            true
        ).unwrap();

        let expected_size = ((qr.size() as u32) + 2 * border) * scale;
        let img = generate_image_buffer(content, Some(border), None, None, Some(scale)).unwrap();

        assert_eq!(img.dimensions(), (expected_size, expected_size));
    }

    #[test]
    fn test_hex_to_rgba() {
        assert_eq!(hex_to_rgba("#FF0000"), Ok([255, 0, 0, 255]));
        assert_eq!(hex_to_rgba("00FF00"), Ok([0, 255, 0, 255]));
        assert_eq!(hex_to_rgba("#FF00007F"), Ok([255, 0, 0, 127]));
        assert_eq!(
            hex_to_rgba("FF00"),
            Err("Hex code must be 6 (RRGGBB) or 8 (RRGGBBAA) characters")
        );
        assert_eq!(hex_to_rgba("GG0000"), Err("Hex code contains invalid characters"));
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#FF0000"), Ok([255, 0, 0]));
        assert_eq!(hex_to_rgb("00FF00"), Ok([0, 255, 0]));
        assert_eq!(hex_to_rgb("FF00"), Err("Hex code must be 6 characters (RRGGBB)"));
        assert_eq!(hex_to_rgb("GG0000"), Err("Hex code contains invalid characters"));
        assert_eq!(hex_to_rgb("FF00007F"), Err("Hex code must be 6 characters (RRGGBB)"));
    }
}
