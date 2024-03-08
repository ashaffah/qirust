use crate::qr_lib::{QrCode, QrCodeEcc, Version};

use image::{ImageBuffer, Luma};
use std::path::Path;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/*---- Utilities ----*/

// Returns a string of SVG code for an image depicting
// the given QR Code, with the given number of border modules.
// The string always uses Unix newlines (\n), regardless of the platform.
pub fn to_svg_string(qr: &QrCode, border: i32) -> String {
	assert!(border >= 0, "Border must be non-negative");
	let mut result = String::new();
	result += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
	result += "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";
	let dimension = qr.size().checked_add(border.checked_mul(2).unwrap()).unwrap();
	result += &format!(
		"<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n", dimension);
	result += "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n";
	result += "\t<path d=\"";
	for y in 0 .. qr.size() {
		for x in 0 .. qr.size() {
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
pub fn print_qr(qr: &QrCode) {
	let border: i32 = 4;
	for y in -border .. qr.size() + border {
		for x in -border .. qr.size() + border {
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
/// qr_to_image_and_save(&qr, Some("images"), Some("qr_code.png")).unwrap();
/// ```
pub fn qr_to_image_and_save(qr: &QrCode, directory_path: Option<&str>, filename: Option<&str>) -> Result<(), image::ImageError> {
    let border: i32 = 4;
    let size = qr.size() as u32 + 2 * border as u32;
    let mut img = ImageBuffer::new(size, size);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let qr_x = x as i32 - border;
        let qr_y = y as i32 - border;
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
        let since_the_epoch = start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        format!("{:?}", since_the_epoch)
    	},
		};

    let file_path = format!("{}/{}.png", directory_path, filename);

    // Check if the directory exists, create it if it doesn't
    if !Path::new(directory_path).exists() {
        fs::create_dir_all(directory_path)?;
    }

    img.save(&Path::new(&file_path))
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
/// generate_image("Hello, World!", Some("images"), Some("qr_code.png"));
/// ```
pub fn generate_image(content: &str, directory: Option<&str>, filename: Option<&str>) {
	let text: &str = content;   // User-supplied Unicode text
	let errcorlvl: QrCodeEcc = QrCodeEcc::Low;  // Error correction level

	// Make and print the QR Code symbol
    let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(text, &mut tempbuffer, &mut outbuffer,
        errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
    // Note: qr has a reference to outbuffer, so outbuffer needs to outlive qr
    std::mem::drop(tempbuffer);  // Optional, because tempbuffer is only needed during encode_text()
    print_qr(&qr);
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
	let text: &str = content;   // User-supplied Unicode text
	let errcorlvl: QrCodeEcc = QrCodeEcc::Low;  // Error correction level

	// Make and print the QR Code symbol
    let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(text, &mut tempbuffer, &mut outbuffer,
        errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
    // Note: qr has a reference to outbuffer, so outbuffer needs to outlive qr
    std::mem::drop(tempbuffer);  // Optional, because tempbuffer is only needed during encode_text()
    print_qr(&qr);
    println!("{}", to_svg_string(&qr, 4));
    to_svg_string(&qr, 4)
}

/// Generates a QR Code image buffer from the provided content.
///
/// # Arguments
///
/// * `content` - The content to encode into the QR Code.
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
/// let img_buffer = generate_image_buffer("Hello, World!");
/// ```
pub fn generate_image_buffer(content: &str) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let text: &str = content;   // User-supplied Unicode text
	let errcorlvl: QrCodeEcc = QrCodeEcc::Low;  // Error correction level

	// Make and print the QR Code symbol
    let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr: QrCode = QrCode::encode_text(text, &mut tempbuffer, &mut outbuffer,
        errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
        std::mem::drop(tempbuffer);  // Optional, because tempbuffer is only needed during encode_text()
	let border: i32 = 4;
    let size = qr.size() as u32 + 2 * border as u32;
    let mut img = ImageBuffer::new(size, size);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let qr_x = x as i32 - border;
        let qr_y = y as i32 - border;
        *pixel = if qr.get_module(qr_x, qr_y) {
            Luma([0u8]) // Black
        } else {
            Luma([255u8]) // White
        };
    }

    img
}

// Tests 
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_svg_string() {
        let errcorlvl: QrCodeEcc = QrCodeEcc::Low;
        let mut outbuffer  = vec![0u8; Version::MAX.buffer_len()];
        let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
        let qr = QrCode::encode_text("HELLO WORLD", &mut tempbuffer, &mut outbuffer,
        errcorlvl, Version::MIN, Version::MAX, None, true).unwrap();
        let svg = to_svg_string(&qr, 4);
        
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn test_generate_image_buffer() {
        let content = "Hello, world!";
        let img = generate_image_buffer(content);

        // The QR code for "Hello, world!" with a low error correction level
        // and a border of 4 should be 29x29 pixels.
        assert_eq!(img.dimensions(), (29, 29));
    }
}