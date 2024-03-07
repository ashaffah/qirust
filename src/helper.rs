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

// Prints the given QrCode object to the console.
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

pub fn generate_image(content: &'static str, directory: Option<&str>, filename: Option<&str>) {
	let text: &'static str = content;   // User-supplied Unicode text
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

pub fn generate_svg_string(content: &'static str) -> String {
	let text: &'static str = content;   // User-supplied Unicode text
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


// pub fn qr_to_image(qr: &QrCode) -> ImageBuffer<Luma<u8>, Vec<u8>> {
// 		let border: i32 = 4;
//     let size = qr.size() as u32 + 2 * border as u32;
//     let mut img = ImageBuffer::new(size, size);

//     for (x, y, pixel) in img.enumerate_pixels_mut() {
//         let qr_x = x as i32 - border;
//         let qr_y = y as i32 - border;
//         *pixel = if qr.get_module(qr_x, qr_y) {
//             Luma([0u8]) // Black
//         } else {
//             Luma([255u8]) // White
//         };
//     }

//     img
// }
