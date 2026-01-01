# qirust

**A Rust library for generating QR codes with customizable rendering and styling options.**

[![Crates.io](https://img.shields.io/crates/v/qirust.svg)](https://crates.io/crates/qirust)
[![Docs.rs](https://docs.rs/qirust/badge.svg)](https://docs.rs/qirust)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`qirust` is a safe and efficient Rust library for generating QR codes, adhering to the QR Code Model 2 specification. It supports encoding text or binary data with customizable error correction levels (Low, Medium, Quartile, High), versions (1 to 40), and output formats including console ASCII art, PNG images, SVGs, and in-memory image buffers. Advanced styling options include logo embedding, custom colors, and square or rounded frames, optimized for performance with horizontal module grouping and caching.

## Features

- **Encoding Modes**: Numeric, alphanumeric, byte, and ECI (Kanji mode defined but not implemented).
- **Error Correction**: Four levels (Low, Medium, Quartile, High) to balance capacity and robustness.
- **Output Formats**: Console ASCII art, PNG images, SVGs, and in-memory image buffers.
- **Styling Options**: Embed logos, customize colors, and apply square or rounded frames behind logos.
- **Safety**: Pure Rust implementation with no unsafe code, ensuring memory safety.
- **Performance**: Optimized with horizontal module grouping, logo caching, and minimal memory allocations.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
qirust = "0.1"
image = "0.25"
```

## Getting Started

Below are examples demonstrating how to use `qirust` to generate QR codes in various formats and styles.

### Example: Styled QR Code with Logo

Generate a QR code with an embedded logo, orange modules, and a rounded frame:

```rust,no_run
use qirust::helper::{generate_frameqr, FrameQrConfig, FrameStyle};
use qirust::qrcode::QrCodeEcc;

fn main() {
    let config = FrameQrConfig::new("logo.png").unwrap()
        .with_ecc(QrCodeEcc::High)
        .with_upscale(6).unwrap()
        .with_directory("output")
        .with_filename("styled_qr")
        .with_color([255, 165, 0])
        .with_outer_frame(40)
        .with_inner_frame(10)
        .with_frame_style(FrameStyle::Rounded);

    generate_frameqr("https://example.com", config)
        .expect("Failed to generate QR code");
}
```

### Example: In-Memory Image Buffer

Generate a QR code as an in-memory image buffer with custom colors:

```rust
use qirust::helper::{generate_image_buffer, QrConfig};

fn main() {
    let config = QrConfig::new()
        .with_border(4).unwrap()
        .with_fg_color([255, 0, 0])
        .with_bg_color([255, 255, 255])
        .with_scale(6).unwrap();

    let img = generate_image_buffer("Hello, World!", config)
        .expect("Failed to generate image buffer");
    img.save("output/qr.png").expect("Failed to save image");
}
```

### Example: SVG with Logo

Generate an SVG string for a styled QR code with a logo:

```rust,no_run
use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
use qirust::helper::{frameqr_to_svg_string, FrameQrSvgConfig, FrameStyle};

fn main() {
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        "https://example.com",
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: QrCodeEcc::High,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    ).unwrap();

    let config = FrameQrSvgConfig::new("logo.png").unwrap()
        .with_upscale(6).unwrap()
        .with_color([255, 165, 0])
        .with_outer_frame(40)
        .with_inner_frame(10)
        .with_frame_style(FrameStyle::Rounded);

    let svg = frameqr_to_svg_string(qr, config)
        .expect("Failed to generate SVG");
    println!("{}", svg);
}
```

### Example: Console Output

Encode text and print the QR code to the console:

```rust
use qirust::qrcode::{QrCode, QrCodeEcc, Version, EncodeTextOptions};
use qirust::helper::print_qr;

fn main() -> Result<(), qirust::qrcode::DataTooLong> {
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    let qr = QrCode::encode_text(
        "Hello, World!",
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: QrCodeEcc::Low,
            minversion: Version::MIN,
            maxversion: Version::MAX,
            mask: None,
            boostecl: true,
        },
    )?;
    print_qr(&qr);
    Ok(())
}
```

## Modules

### Module: `qrcode`

Handles core QR code encoding, supporting versions 1 to 40 and four error correction levels.

#### Structs

- [**`QrCode`**]: Represents a QR code grid of dark and light modules.
- [**`QrCodeEcc`**]: Defines error correction levels (Low, Medium, Quartile, High).
- [**`QrSegment`**]: Represents a data segment (numeric, alphanumeric, byte, or ECI).
- [**`Version`**]: Specifies QR code version (1–40).
- [**`Mask`**]: Defines mask patterns (0–7).
- [**`EncodeTextOptions`**]: Configuration options for text encoding.

#### Key Functions

- [**`QrCode::encode_text`**]: Encodes a text string into a QR code with specified options.
- [**`QrCode::encode_binary`**]: Encodes binary data into a QR code.
- [**`QrSegment::make_numeric`**]: Creates a numeric mode segment.
- [**`QrSegment::make_alphanumeric`**]: Creates an alphanumeric mode segment.
- [**`QrSegment::make_bytes`**]: Creates a byte mode segment.

#### Documentation

See the [qrcode module documentation](https://docs.rs/qirust/latest/qirust/qrcode/index.html) for details.

### Module: `helper`

Provides utilities for rendering QR codes in various formats with styling options, including logo embedding and custom frames.

#### Configuration Structs

- [**`QrConfig`**]: Configuration for basic QR code rendering (border, colors, scale).
- [**`FrameQrConfig`**]: Configuration for styled QR codes with frames and logos (for saving to disk).
- [**`FrameQrSaveConfig`**]: Internal configuration for saving styled QR codes.
- [**`FrameQrSvgConfig`**]: Configuration for SVG styled QR codes with logos.
- [**`FrameStyle`**]: Enum for frame styles (None, Square, Rounded).

#### Key Functions

- [**`print_qr`**]: Displays a QR code in the console using ASCII characters.
- [**`to_svg_string`**]: Generates an SVG string for a QR code.
- [**`qr_to_image_and_save`**]: Saves a basic QR code as a PNG image.
- [**`frameqr_to_image_and_save`**]: Saves a styled QR code with a logo and optional frames.
- [**`frameqr_to_svg_string`**]: Generates an SVG string for a styled QR code with a logo.
- [**`generate_frameqr`**]: Convenience function to generate a styled QR code from text.
- [**`generate_image`**]: Saves a basic QR code as a PNG.
- [**`generate_svg_string`**]: Generates an SVG string from text.
- [**`generate_image_buffer`**]: Creates an in-memory QR code image buffer.
- [**`generate_frameqr_buffer`**]: Creates an in-memory image buffer for a styled QR code with a logo.
- [**`mix_colors`**]: Blends colors for rendering smooth transitions.
- [**`encode_base64`**]: Encodes bytes into a base64 string for logo embedding.
- [**`hex_to_rgba`**]: Converts a hex color code to an RGBA array.
- [**`hex_to_rgb`**]: Converts a hex color code to an RGB array.

#### Documentation

See the [helper module documentation](https://docs.rs/qirust/latest/qirust/helper/index.html) for details.

## Error Handling

The library returns errors for specific cases:

- [**`qrcode::DataTooLong`**]: Indicates data exceeds the QR code's capacity. Handle by reducing data size, increasing version, or lowering error correction.
- [**`helper::HelperError`**]: Wraps various errors including image processing, I/O, and validation errors.
- [**`image::ImageError`**]: Occurs for image processing or file I/O errors (e.g., invalid paths or permissions).

Example of handling errors:

```rust
use qirust::qrcode::{QrCode, QrCodeEcc, Version, DataTooLong, EncodeTextOptions};

fn main() {
    let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];

    match QrCode::encode_text(
        "Too long data for this version",
        &mut tempbuffer,
        &mut outbuffer,
        EncodeTextOptions {
            ecl: QrCodeEcc::Low,
            minversion: Version::MIN,
            maxversion: Version::new(5),
            mask: None,
            boostecl: true,
        },
    ) {
        Ok(qr) => println!("QR code generated successfully"),
        Err(DataTooLong::SegmentTooLong) => eprintln!("Segment too long"),
        Err(DataTooLong::DataOverCapacity(datalen, capacity)) => {
            eprintln!("Data length {} exceeds capacity {}", datalen, capacity);
        }
    }
}
```

## Performance

`qirust` is optimized for efficiency:

- **Horizontal Module Grouping**: Reduces rendering complexity in SVG and image outputs, improving performance for high-version QR codes (e.g., Version 40).
- **Logo Caching**: Uses global `Mutex`-based caches for resized logos and base64-encoded images, minimizing redundant processing.
- **Memory Efficiency**: Precomputes buffer sizes and uses minimal allocations for encoding and rendering.
- **Fast Resizing**: Uses `FilterType::Nearest` or `FilterType::Triangle` for logo resizing, balancing speed and quality.

For high-version QR codes or frequent rendering, functions like `generate_frameqr_buffer` and `to_svg_string` are particularly efficient due to these optimizations.

## Limitations

- **Kanji Mode**: Defined but not fully implemented in the `qrcode` module.
- **ECI Mode**: Supported but requires careful handling for non-standard character sets.
- **Logo Size**: Automatically resized to one-third of QR code dimensions for scannability (up to 40% for SVG outputs).
- **File I/O**: Requires valid paths and permissions for image saving.

## Contributing

Contributions are welcome! Please fork the repository, create a feature branch, and submit a pull request with tests and documentation updates. Ensure your code adheres to Rust's safety guidelines and includes appropriate tests.

## License

MIT License. See [LICENSE](https://github.com/ashaffah/qirust/blob/main/LICENSE) for details.
