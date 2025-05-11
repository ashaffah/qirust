//! # qirust
//!
//! A Rust library for generating QR codes with customizable rendering options.
//!
//! `qirust` enables encoding text or binary data into QR codes, adhering to the QR Code Model 2
//! specification. It supports versions 1 to 40, four error correction levels, and various output
//! formats (console, PNG, SVG). The library also offers styling features like logo embedding, custom
//! colors, and frame styles.
//!
//! ## Features
//!
//! - Encode data in numeric, alphanumeric, byte, or ECI modes.
//! - Support four error correction levels: Low, Medium, Quartile, High.
//! - Render QR codes as ASCII art, PNG images, or SVGs.
//! - Style QR codes with logos, custom colors, and square/rounded frames.
//! - Safe Rust implementation with no unsafe code.
//!
//! ## Installation
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! qirust = "0.1" # Replace with the latest version
//! ```
//!
//! ## Example
//!
//! Generate a styled QR code with a logo:
//!
//! ```rust
//! use qirust::{helper::generate_frameqr, qrcode::QrCodeEcc};
//!
//! fn main() {
//!     generate_frameqr(
//!         "https://example.com",
//!         "src/logo.png",
//!         Some(QrCodeEcc::High),
//!         Some(6),
//!         Some("output"),
//!         Some("styled_qr"),
//!         Some([255, 165, 0]), // Orange
//!         Some(40),
//!         Some("rounded"),
//!     );
//! }
//! ```
//!
//! ## Modules
//!
//! - [`qrcode`]: Core QR code encoding functionality.
//! - [`helper`]: Utilities for rendering QR codes in various formats.

pub mod qrcode;
pub mod helper;
