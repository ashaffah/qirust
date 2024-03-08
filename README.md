# qirust

A simple QR code generator written in Rust using standard library.

## Contents

- [Installation](#installation)
- [Usage](#usage)
- [License](#license)

## Installation

1. Add qirust to your cargo.toml

```bash
[dependencies]
qirust = "0.1.5"
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

### generate_image_buffer(content)

content parameter is required

```rust
generate_image_buffer(content: &str) -> ImageBuffer<Luma<u8>, Vec<u8>>
```

### Example :

```rust
use qirust::helper::{generate_image, generate_svg_string};

fn main() {
    generate_image("hello world", None, None); // generate_image("hello world", Some("your_image_directory"), Some("image_name"));
    generate_svg_string("hello world");
    generate_image_buffer("hello world")
}
```

## Lisence

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
