# qirust

[![crates.io](https://img.shields.io/crates/v/qirust.svg)](https://crates.io/crates/qirust)

A simple QR code generator written in Rust using standard library.

## Contents

- [Installation](#installation)
- [Usage](#usage)
- [License](#license)
- [Example](#example)

## Installation

1. Add qirust to your cargo.toml

```bash
[dependencies]
qirust = "0.1.8"
```

2. Then run cargo build command

```bash
cargo build
```

## Usage

### generate_svg_string(content)

```rust
generate_svg_string(content: &str) -> String
```

content parameter is required

### generate_image(content, directory, filename)

```rust
generate_image(content: &str, directory: Option<&str>, filename: Option<&str>)
```

content parameter is required, directory, and filename are optional, if you prefer to use default option set as None

```rust
generate_image("hello world", None, None);
```

### mix_colors(pixel, foreground, background)

```rust
mix_colors(pixel: u8, foreground: u8, background: u8) -> u8
```

pixel, foreground, and background parameters are required

### generate_image_buffer(content, border, fg_color, bg_color)

```rust
generate_image_buffer(
    content: &str,
    border: Option<i32>,
    fg_color: Option<Rgb<u8>>,
    bg_color: Option<Rgb<u8>>
) -> ImageBuffer<Rgb<u8>, Vec<u8>>
```

content parameter is required, border, fg_color, and bg_color are optional, if you prefer to use default option set as None

## Example

```rust
use qirust::helper::{generate_image, generate_svg_string};

fn main() {
    generate_image("hello world", None, None); // generate_image("hello world", Some("your_image_directory"), Some("image_name"));
    generate_svg_string("hello world");
}
```

or you can customize the appearance of the generated QR code as desired using the `generate_image_buffer` function.

```rust
use qirust::helper::generate_image_buffer;
use image::Rgb;

fn main() {
    let foreground_color = Rgb([255, 0, 0]); // Red foreground color
    let background_color = Rgb([0, 0, 0]); // Black background color
    let colored_image_buffer = generate_image_buffer(
        "Hello, World!",
        None,
        Some(foreground_color),
        Some(background_color)
    );

    let colored_image_path = "./colored_image.png";
    colored_image_buffer.save(colored_image_path).unwrap();
}

```

## License

MIT License

Copyright (c) 2024 Ashaffah

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

### <a href="#qirust">Back to top</a>
