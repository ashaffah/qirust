# qirust

**A Rust library for generating QR codes with customizable rendering options.**

[![Crates.io](https://img.shields.io/crates/v/qirust.svg)](https://crates.io/crates/qirust)
[![Docs.rs](https://docs.rs/qirust/badge.svg)](https://docs.rs/qirust)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`qirust` is a safe and efficient Rust library for generating QR codes, adhering to the QR Code Model 2 specification. It supports encoding text or binary data with customizable error correction levels, versions, and styling options, including logo embedding, custom colors, and frame styles. The library provides flexible output formats: console ASCII art, PNG images, and SVGs.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
qirust = "0.1" # Replace with the latest version
image = "0.24" # Required for image rendering
```

## Modules

- [**`qrcode`**](#module-qrcode): Core QR code encoding functionality.
- [**`helper`**](#module-helper): Utilities for rendering QR codes in various formats.

### Module: `qrcode`

Handles QR code encoding, supporting versions 1 to 40 and four error correction levels: `Low`, `Medium`, `Quartile`, and `High`.

#### Structs

- **`QrCode`**: Represents a QR code grid of dark and light modules.
- **`QrCodeEcc`**: Defines error correction levels.
- **`QrSegment`**: Represents a data segment (numeric, alphanumeric, byte, or ECI).
- **`Version`**: Specifies QR code version (1–40).
- **`Mask`**: Defines mask patterns (0–7).

#### Key Functions

- **`QrCode::encode_text`**: Encodes text into a QR code, selecting the smallest version within the specified range.
- **`QrCode::encode_binary`**: Encodes binary data into a QR code.
- **`QrSegment::make_numeric`**, **`make_alphanumeric`**, **`make_bytes`**: Creates optimized data segments.

#### Example

```rust
use qirust::qrcode::{QrCode, QrCodeEcc, Version};

fn main() -> Result<(), qirust::qrcode::DataTooLong> {
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];

    let qr = QrCode::encode_text(
        "Hello, World!",
        &mut tempbuffer,
        &mut outbuffer,
        QrCodeEcc::Low,
        Version::MIN,
        Version::MAX,
        None,
        true,
    )?;

    println!("Version: {}", qr.version().value());
    Ok(())
}
```

### Module: `helper`

Provides utilities for rendering QR codes as console output, PNG images, or SVGs, with styling options like logo embedding and custom colors.

#### Key Functions

- **`print_qr`**: Prints a QR code to the console using ASCII characters.
- **`to_svg_string`**: Generates an SVG string for a QR code.
- **`qr_to_image_and_save`**: Saves a QR code as a PNG image.
- **`frameqr_to_image_and_save`**: Saves a styled QR code with a logo, custom colors, and optional frames.
- **`generate_frameqr`**: Convenience function to generate a styled QR code from text.
- **`generate_image`**: Saves a basic QR code as a PNG.
- **`generate_svg_string`**: Generates an SVG string from text.
- **`generate_image_buffer`**: Creates an in-memory QR code image buffer.
- **`mix_colors`**: Blends foreground and background colors for rendering.

#### Example: Styled QR Code with Logo

```rust
use qirust::{helper::generate_frameqr, qrcode::QrCodeEcc};

fn main() {
    generate_frameqr(
        "https://example.com",
        "logo.png",
        Some(QrCodeEcc::High),
        Some(6),
        Some("output"),
        Some("styled_qr"),
        Some([255, 165, 0]), // Orange
        Some(40), // Frame size
        Some("rounded"), // Frame style
    ).expect("Failed to generate QR code");
}
```

#### Example: SVG Generation

```rust
use qirust::helper::generate_svg_string;

fn main() {
    let svg = generate_svg_string("Hello, World!");
    println!("{}", svg);
}
```

## Error Handling

- **`qrcode::DataTooLong`**: Indicates data exceeds the QR code’s capacity. Handle by reducing data size, increasing version, or lowering error correction.
- **`image::ImageError`**: Returned for image processing or file I/O errors (e.g., invalid paths).

```rust
match QrCode::encode_text(...) {
    Ok(qr) => println!("QR code generated"),
    Err(qirust::qrcode::DataTooLong::SegmentTooLong) => eprintln!("Data too long"),
    Err(qirust::qrcode::DataTooLong::DataOverCapacity(datalen, capacity)) => {
        eprintln!("Data length {} exceeds capacity {}", datalen, capacity);
    }
}
```

## Features

- **Encoding Modes**: Numeric, alphanumeric, byte, and ECI (Kanji mode defined but unimplemented).
- **Error Correction**: Four levels to balance capacity and robustness.
- **Output Formats**: Console, PNG, SVG.
- **Styling**: Logo embedding, custom colors, square/rounded frames.
- **Safety**: No unsafe code, adhering to Rust’s safety guarantees.
- **Testing**: Unit tests for SVG and image buffer generation.

## Limitations

- **Kanji Mode**: Defined but not fully implemented.
- **ECI Mode**: Supported but requires careful use.
- **Logo Size**: Automatically resized to one-third of QR code dimensions for scannability.
- **File I/O**: Requires valid paths and permissions for image saving.

## Contributing

Contributions are welcome! Fork the repository, create a feature branch, and submit a pull request with tests and documentation.

## License

MIT License. See [LICENSE](https://github.com/your-repo/qirust/blob/main/LICENSE) for details.
