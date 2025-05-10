use crate::qrcode::{ QrCode, QrCodeEcc, Version };

use image::{
    imageops::{ overlay, resize, FilterType },
    DynamicImage,
    ImageBuffer,
    Luma,
    Rgb,
    RgbaImage,
};
use std::{ env, fs, path::{ Path, PathBuf }, time::{ SystemTime, UNIX_EPOCH } };

/*---- Utilities ----*/

// Returns a string of SVG code for an image depicting
// the given QR Code, with the given number of border modules.
// The string always uses Unix newlines (\n), regardless of the platform.
pub fn to_svg_string(qr: &QrCode, border: i32) -> String {
    assert!(border >= 0, "Border must be non-negative");
    let mut result = String::new();
    result += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    result +=
        "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";
    let dimension = qr.size().checked_add(border.checked_mul(2).unwrap()).unwrap();
    result += &format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n",
        dimension
    );
    result += "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n";
    result += "\t<path d=\"";
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            if qr.get_module(x, y) {
                if x != 0 || y != 0 {
                    result += " ";
                }
                result += &format!("M{},{}h1v1h-1z", x + border, y + border);
            }
        }
    }
    result += "\" fill=\"#000000\"/>\n";
    result += "</svg>\n";
    result
}

/// Prints the given QrCode object to the console.
///
/// # Argument
///
/// * `qr` - The QrCode object to print.
///
/// # Example
///
/// ```rust
/// use qirust::helper::print_qr;
/// use qirust::qr_lib::{QrCode, QrCodeEcc, Version};
///
/// let errcorlvl: QrCodeEcc = QrCodeEcc::Low;  // Error correction level
/// let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = qirust::qr_lib::QrCode::encode_text("Hello, World!", &mut tempbuffer, &mut outbuffer,
///    errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
/// std::mem::drop(tempbuffer);
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

/// Converts a QR Code object to an image and saves it to a file.
///
/// # Arguments
///
/// * `qr` - The QR Code object to convert.
/// * `directory_path` - Optional. The directory path where the image will be saved. If not provided, the default directory is "generated".
/// * `filename` - Optional. The name of the image file. If not provided, a timestamp-based filename will be used.
///
/// # Errors
///
/// Returns an `image::ImageError` if there is an error saving the image.
///
/// # Example
///
/// ```rust
/// use qirust::helper::qr_to_image_and_save;
/// use qirust::qr_lib::{QrCode, QrCodeEcc, Version};
///
/// let errcorlvl: QrCodeEcc = QrCodeEcc::Low;  // Error correction level
/// let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let qr = qirust::qr_lib::QrCode::encode_text("Hello, World!", &mut tempbuffer, &mut outbuffer,
///     errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
/// std::mem::drop(tempbuffer);
///
/// qr_to_image_and_save(&qr, Some("images"), Some("qr_code")).unwrap();
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

    let directory_path = directory_path.unwrap_or("generated");
    let filename = match filename {
        Some(name) => name.to_string(),
        None => {
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
            format!("{:?}", since_the_epoch)
        }
    };

    let file_path = format!("{}/{}.png", directory_path, filename);

    // Check if the directory exists, create it if it doesn't
    if !Path::new(directory_path).exists() {
        fs::create_dir_all(directory_path)?;
    }

    img.save(&Path::new(&file_path))
}

/// Converts a QR Code into an image with an embedded logo, optional frame styles, coloring, and scaling,
/// then saves it as a PNG file.
///
/// # Arguments
///
/// * `qr` - The QR Code object to convert.
/// * `logo_path` - **Required.** Path to the logo image file to embed at the center of the QR Code.
/// * `upscale_factor` - Optional. Scaling factor for enlarging the QR code (default is 4).
/// * `directory_path` - Optional. Output directory to save the image (default is "generated").
/// * `file_name` - Optional. Filename (without extension). If not provided, a timestamp is used.
/// * `qr_color` - Optional. RGB color of the dark modules (default is black `[0, 0, 0]`).
/// * `outer_frame_px` - Optional. Size of white outer padding in pixels (default is none).
/// * `frame_style` - Optional. `"rounded"` or `"square"` background frame behind the logo.
///
/// # Returns
///
/// * `Result<(), image::ImageError>` on success or failure during image generation/saving.
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{ QrCode, QrCodeEcc, Version };
/// use qirust::helper::frameqr_to_image_and_save;
///
/// let content = "your content here";
/// let logo_path = "src/logo.png";
/// let errcorlvl = QrCodeEcc::High;
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
///
/// // Colored + outer frame
/// let mut out1 = outbuffer;
/// let mut temp1 = tempbuffer;
/// let qr = QrCode::encode_text(
///     content,
///     &mut temp1,
///     &mut out1,
///     errcorlvl,
///     Version::new(10),
///     Version::new(30),
///     None,
///     true
/// ).unwrap();
/// frameqr_to_image_and_save(
///     qr,
///     logo_path,
///     Some(4),
///     None,
///     None,
///     Some([255, 165, 0]),
///     Some(40),
///     None
/// ).unwrap();
/// ```
pub fn frameqr_to_image_and_save(
    qr: QrCode,
    logo_path: &str,
    upscale_factor: Option<u32>,
    directory_path: Option<&str>,
    file_name: Option<&str>,
    qr_color: Option<[u8; 3]>,
    outer_frame_px: Option<u32>,
    frame_style: Option<&str> // "square", "rounded"
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

    let upscale = upscale_factor.unwrap_or(4);
    let mut upscaled_qr = resize(
        &DynamicImage::ImageRgb8(qr_img),
        qr_size * upscale,
        qr_size * upscale,
        FilterType::Nearest
    );

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let full_path: PathBuf = current_dir.join(logo_path);
    println!("Logo path: {:?}", full_path);

    let logo = image::open(&Path::new(&full_path))?.to_rgba8();
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
    match frame_style {
        Some("rounded") => {
            let frame_margin = 3;
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
        Some("square") => {
            let frame_margin = 3;
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
        _ => {}
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

/// Generates a styled QR Code image with an embedded logo and saves it to a PNG file.
///
/// This function wraps [`frameqr_to_image_and_save`] for convenience, allowing quick generation of
/// logo-enhanced QR codes with optional styling (color, quiet zone, rounded/square frame).
///
/// # Arguments
///
/// * `content` - The string content to encode into the QR Code.
/// * `logo_path` - **Required.** File path to the logo image that will be placed in the center of the QR.
/// * `ecc` - Optional. Error correction level (`QrCodeEcc::Low`, `Medium`, `Quartile`, or `High`). Defaults to `High`.
/// * `upscale_factor` - Optional. Pixel scale multiplier for output image size (default is 4).
/// * `directory_path` - Optional. Directory to save the PNG file (default is "generated").
/// * `file_name` - Optional. Name of the PNG file (without extension). If not provided, uses a timestamp.
/// * `qr_color` - Optional. RGB color array (e.g., `[255, 0, 0]` for red). Defaults to black.
/// * `outer_frame_px` - Optional. Quiet zone padding around the QR code in pixels.
/// * `frame_style` - Optional. `"rounded"` or `"square"` style background frame behind the logo.
///
/// # Example
///
/// ```rust
/// use qirust::{ helper::generate_frameqr, qrcode::QrCodeEcc };
///
/// fn main() {
///     let content = "your content here";
///     let logo_path = "src/logo.png";
///
///     let errcorlvl: QrCodeEcc = QrCodeEcc::High;
///
///     let qr_color = [255, 165, 0]; // orange
///
///     let outer_frame_px = 40; // ~25% quiet zone
///     let frame_style = Some("rounded");
///
///     generate_frameqr(
///         content,
///         logo_path,
///         Some(errcorlvl),
///         Some(6),
///         None,
///         None,
///         Some(qr_color),
///         Some(outer_frame_px),
///         frame_style
///     );
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
    frame_style: Option<&str>
) {
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
        frame_style
    ).unwrap()
}

/// Generates a QR Code image from the provided content and saves it to a file.
///
/// # Arguments
///
/// * `content` - The content to encode into the QR Code.
/// * `directory` - Optional. The directory path where the image will be saved. If not provided, the default directory is "generated".
/// * `filename` - Optional. The name of the image file. If not provided, a timestamp-based filename will be used.
///
/// # Example
///
/// ```
/// use qirust::helper::generate_image;
///
/// generate_image("Hello, World!", Some("images"), Some("qr_code"));
/// ```
pub fn generate_image(content: &str, directory: Option<&str>, filename: Option<&str>) {
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
    // Note: qr has a reference to outbuffer, so outbuffer needs to outlive qr
    std::mem::drop(tempbuffer); // Optional, because tempbuffer is only needed during encode_text()
    qr_to_image_and_save(&qr, directory, filename).unwrap();
}

/// Generates a QR Code SVG from the provided content.
///
/// # Arguments
///
/// * `content` - The content to encode into the QR Code.
///
/// # Returns
///
/// A string of SVG code representing the QR Code image.
///
/// # Example
///
/// ```
/// use qirust::helper::generate_svg_string;
///
/// let svg_string = generate_svg_string("Hello, World!");
/// println!("{}", svg_string);
/// ```
pub fn generate_svg_string(content: &str) -> String {
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
    // Note: qr has a reference to outbuffer, so outbuffer needs to outlive qr
    std::mem::drop(tempbuffer); // Optional, because tempbuffer is only needed during encode_text()
    to_svg_string(&qr, 4)
}

/// Mixes the foreground and background colors based on the pixel value.
///
/// # Arguments
///
/// * `pixel` - The pixel value.
/// * `foreground` - The foreground color value.
/// * `background` - The background color value.
///
/// # Returns
///
/// The mixed color value.
///
/// # Example
///
/// ```rust
/// use qirust::helper::mix_colors;
/// use image::Rgb;
///
/// let pixel = 128;
/// let foreground = Rgb([255, 0, 0]); // Red
/// let background = Rgb([0, 0, 255]); // Blue
///
/// let mixed_color = mix_colors(pixel, foreground[0], background[0]);
/// println!("{}", mixed_color);
/// ```
pub fn mix_colors(pixel: u8, foreground: u8, background: u8) -> u8 {
    (((pixel as u16) * (foreground as u16)) / 255 +
        ((255 - (pixel as u16)) * (background as u16)) / 255) as u8
}

/// Generates a QR Code image buffer from the provided content.
///
/// # Arguments
///
/// * `content` - The content to encode into the QR Code.
/// * `border` - Optional. The number of border modules to add around the QR Code. If not provided, the default is 4.
/// * `fg_color` - Optional. The foreground color of the QR Code. If not provided, the default is black.
/// * `bg_color` - Optional. The background color of the QR Code. If not provided, the default is white.
///
/// # Returns
///
/// An `ImageBuffer` representing the QR Code image.
///
/// # Example
///
/// ```
/// use qirust::helper::generate_image_buffer;
///
/// let img_buffer = generate_image_buffer("Hello, World!", None, None, None); // Border is 4 by default and colors are black and white by default
/// ```
pub fn generate_image_buffer(
    content: &str,
    border: Option<i32>,
    fg_color: Option<Rgb<u8>>,
    bg_color: Option<Rgb<u8>>
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let border = border.unwrap_or(4);
    let foreground_color = fg_color.unwrap_or(Rgb([0, 0, 0]));
    let background_color = bg_color.unwrap_or(Rgb([255, 255, 255]));
    let errcorlvl = QrCodeEcc::Low;

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

    let size = (qr.size() + 2 * border) as u32;
    let mut img = ImageBuffer::new(size, size);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let qr_x = (x as i32) - border;
        let qr_y = (y as i32) - border;
        *pixel = if qr.get_module(qr_x, qr_y) { foreground_color } else { background_color };
    }

    // Apply color mixing here
    let colored_image_buffer = ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let pixel = img.get_pixel(x, y);
        Rgb([
            mix_colors(pixel[0], foreground_color[0], background_color[0]),
            mix_colors(pixel[0], foreground_color[1], background_color[1]),
            mix_colors(pixel[0], foreground_color[2], background_color[2]),
        ])
    });

    colored_image_buffer
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
        let img = generate_image_buffer(content, None, None, None);

        // The QR code for "Hello, world!" with a low error correction level
        // and a border of 4 should be 29x29 pixels.
        assert_eq!(img.dimensions(), (29, 29));
    }
}
