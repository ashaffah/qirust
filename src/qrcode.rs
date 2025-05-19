#![forbid(unsafe_code)]
#![allow(unused_assignments)]
#![allow(dead_code)]
/// QR code encoding functionality.
///
/// This module provides the core logic for encoding data into QR codes, supporting the QR Code Model
/// 2 specification. It includes structs and functions for creating QR codes with customizable versions
/// (1–40), error correction levels, and data modes (numeric, alphanumeric, byte, ECI).
use core::convert::TryFrom;

/// A QR Code symbol, representing a square grid of dark and light modules.
///
/// This struct supports QR Code Model 2, covering versions 1 to 40, all four error correction levels,
/// and four encoding modes (numeric, alphanumeric, byte, ECI). Instances are immutable after creation.
///
/// # Creation
///
/// - High-level: Use [`encode_text`] or [`encode_binary`].
/// - Mid-level: Use [`encode_segments_to_codewords`] and [`encode_codewords`].
/// - Low-level: Directly construct with [`encode_codewords`].
///
/// # Example
///
/// ```rust
/// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
///
/// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
/// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
///
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
/// println!("Version: {}", qr.version().value());
/// ```
pub struct QrCode<'a> {
    /// The width and height of this QR Code, measured in modules, between
    /// 21 and 177 (inclusive). This is equal to version * 4 + 17.
    size: &'a mut u8,

    /// The modules of this QR Code (0 = light, 1 = dark), packed bitwise into bytes.
    /// Immutable after constructor finishes. Accessed through get_module().
    modules: &'a mut [u8],
}

impl<'a> QrCode<'a> {
    /// Encodes a text string into a QR code.
    ///
    /// Automatically selects the smallest version within the given range that can hold the data.
    /// If `boostecl` is `true`, the error correction level may be increased if it doesn't increase
    /// the version. The `mask` can be `None` for automatic selection (slower) or a value from 0 to 7.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode.
    /// * `tempbuffer` - Temporary buffer, at least [`Version::MAX.buffer_len`] bytes.
    /// * `outbuffer` - Output buffer, at least [`Version::MAX.buffer_len`] bytes.
    /// * `ecl` - Error correction level.
    /// * `minversion` - Minimum QR code version.
    /// * `maxversion` - Maximum QR code version.
    /// * `mask` - Optional mask pattern.
    /// * `boostecl` - Whether to boost error correction if possible.
    ///
    /// # Returns
    ///
    /// A `Result` containing the QR code or a [`DataTooLong`] error if the data is too long.
    ///
    /// # Example
    ///
    /// ```rust
    /// use qirust::qrcode::{QrCode, QrCodeEcc, Version};
    ///
    /// let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
    /// let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
    ///
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
    /// ```
    pub fn encode_text<'b>(
        text: &str,
        tempbuffer: &'b mut [u8],
        mut outbuffer: &'a mut [u8],
        ecl: QrCodeEcc,
        minversion: Version,
        maxversion: Version,
        mask: Option<Mask>,
        boostecl: bool
    ) -> Result<QrCode<'a>, DataTooLong> {
        let minlen: usize = outbuffer.len().min(tempbuffer.len());
        outbuffer = &mut outbuffer[..minlen];

        let textlen: usize = text.len(); // In bytes
        if textlen == 0 {
            let (datacodewordslen, ecl, version) = QrCode::encode_segments_to_codewords(
                &[],
                outbuffer,
                ecl,
                minversion,
                maxversion,
                boostecl
            )?;
            return Ok(
                Self::encode_codewords(outbuffer, datacodewordslen, tempbuffer, ecl, version, mask)
            );
        }

        use QrSegmentMode::*;
        let buflen: usize = outbuffer.len();
        let seg: QrSegment = if
            QrSegment::is_numeric(text) &&
            QrSegment::calc_buffer_size(Numeric, textlen).map_or(false, |x| x <= buflen)
        {
            QrSegment::make_numeric(text, tempbuffer)
        } else if
            QrSegment::is_alphanumeric(text) &&
            QrSegment::calc_buffer_size(Alphanumeric, textlen).map_or(false, |x| x <= buflen)
        {
            QrSegment::make_alphanumeric(text, tempbuffer)
        } else if QrSegment::calc_buffer_size(Byte, textlen).map_or(false, |x| x <= buflen) {
            QrSegment::make_bytes(text.as_bytes())
        } else {
            return Err(DataTooLong::SegmentTooLong);
        };
        let (datacodewordslen, ecl, version) = QrCode::encode_segments_to_codewords(
            &[seg],
            outbuffer,
            ecl,
            minversion,
            maxversion,
            boostecl
        )?;
        Ok(Self::encode_codewords(outbuffer, datacodewordslen, tempbuffer, ecl, version, mask))
    }

    /// Encodes binary data into a QR code.
    ///
    /// Similar to [`encode_text`], but for arbitrary byte data. The input data must fit within the
    /// specified version range and error correction level.
    ///
    /// # Arguments
    ///
    /// * `dataandtempbuffer` - Buffer containing data (first `datalen` bytes) and temporary space.
    /// * `datalen` - Length of the data to encode.
    /// * `outbuffer` - Output buffer, at least [`Version::MAX.buffer_len`] bytes.
    /// * `ecl` - Error correction level.
    /// * `minversion` - Minimum QR code version.
    /// * `maxversion` - Maximum QR code version.
    /// * `mask` - Optional mask pattern.
    /// * `boostecl` - Whether to boost error correction if possible.
    ///
    /// # Returns
    ///
    /// A `Result` containing the QR code or a [`DataTooLong`] error.
    pub fn encode_binary<'b>(
        dataandtempbuffer: &'b mut [u8],
        datalen: usize,
        mut outbuffer: &'a mut [u8],
        ecl: QrCodeEcc,
        minversion: Version,
        maxversion: Version,
        mask: Option<Mask>,
        boostecl: bool
    ) -> Result<QrCode<'a>, DataTooLong> {
        assert!(datalen <= dataandtempbuffer.len(), "Invalid data length");
        let minlen: usize = outbuffer.len().min(dataandtempbuffer.len());
        outbuffer = &mut outbuffer[..minlen];

        if
            QrSegment::calc_buffer_size(QrSegmentMode::Byte, datalen).map_or(
                true,
                |x| x > outbuffer.len()
            )
        {
            return Err(DataTooLong::SegmentTooLong);
        }
        let seg: QrSegment = QrSegment::make_bytes(&dataandtempbuffer[..datalen]);
        let (datacodewordslen, ecl, version) = QrCode::encode_segments_to_codewords(
            &[seg],
            outbuffer,
            ecl,
            minversion,
            maxversion,
            boostecl
        )?;
        Ok(
            Self::encode_codewords(
                outbuffer,
                datacodewordslen,
                dataandtempbuffer,
                ecl,
                version,
                mask
            )
        )
    }

    /// Returns an intermediate state representing the given segments
    /// with the given encoding parameters being encoded into codewords.
    ///
    /// The smallest possible QR Code version within the given range is automatically
    /// chosen for the output. If `boostecl` is `true`, the ECC level may be higher than the
    /// `ecl` argument if it can be done without increasing the version. The `mask` can be
    /// `None` for automatic selection or a value from 0 to 7.
    ///
    /// # Arguments
    ///
    /// * `segs` - Array of segments to encode.
    /// * `outbuffer` - Output buffer for codewords.
    /// * `ecl` - Error correction level.
    /// * `minversion` - Minimum QR code version.
    /// * `maxversion` - Maximum QR code version.
    /// * `boostecl` - Whether to boost error correction if possible.
    ///
    /// # Returns
    ///
    /// A `Result` containing the length of data codewords, updated ECL, and version, or a [`DataTooLong`] error.
    pub fn encode_segments_to_codewords(
        segs: &[QrSegment],
        outbuffer: &'a mut [u8],
        mut ecl: QrCodeEcc,
        minversion: Version,
        maxversion: Version,
        boostecl: bool
    ) -> Result<(usize, QrCodeEcc, Version), DataTooLong> {
        assert!(minversion <= maxversion, "Invalid value");
        assert!(
            outbuffer.len() >= QrCode::get_num_data_codewords(maxversion, ecl),
            "Invalid buffer length"
        );

        // Find the minimal version number to use
        let mut version: Version = minversion;
        let datausedbits: usize = loop {
            let datacapacitybits: usize = QrCode::get_num_data_codewords(version, ecl) * 8;
            let dataused: Option<usize> = QrSegment::get_total_bits(segs, version);
            if dataused.map_or(false, |n| n <= datacapacitybits) {
                break dataused.unwrap();
            } else if version >= maxversion {
                return Err(match dataused {
                    None => DataTooLong::SegmentTooLong,
                    Some(n) => DataTooLong::DataOverCapacity(n, datacapacitybits),
                });
            } else {
                version = Version::new(version.value() + 1);
            }
        };

        // Increase the error correction level while the data still fits
        for &newecl in &[QrCodeEcc::Medium, QrCodeEcc::Quartile, QrCodeEcc::High] {
            if boostecl && datausedbits <= QrCode::get_num_data_codewords(version, newecl) * 8 {
                ecl = newecl;
            }
        }

        // Concatenate all segments to create the data bit string
        let datacapacitybits: usize = QrCode::get_num_data_codewords(version, ecl) * 8;
        let mut bb = BitBuffer::new(&mut outbuffer[..datacapacitybits / 8]);
        for seg in segs {
            bb.append_bits(seg.mode.mode_bits(), 4);
            bb.append_bits(
                u32::try_from(seg.numchars).unwrap(),
                seg.mode.num_char_count_bits(version)
            );
            for i in 0..seg.bitlength {
                let bit: u8 = (seg.data[i >> 3] >> (7 - (i & 7))) & 1;
                bb.append_bits(bit.into(), 1);
            }
        }
        debug_assert_eq!(bb.length, datausedbits);

        // Add terminator and pad up to a byte if applicable
        let numzerobits: usize = core::cmp::min(4, datacapacitybits - bb.length);
        bb.append_bits(0, u8::try_from(numzerobits).unwrap());
        let numzerobits: usize = bb.length.wrapping_neg() & 7;
        bb.append_bits(0, u8::try_from(numzerobits).unwrap());
        debug_assert_eq!(bb.length % 8, 0);

        // Pad with alternating bytes until data capacity is reached
        for &padbyte in [0xec, 0x11].iter().cycle() {
            if bb.length >= datacapacitybits {
                break;
            }
            bb.append_bits(padbyte, 8);
        }
        Ok((bb.length / 8, ecl, version))
    }

    /// Creates a new QR Code with the given version number,
    /// error correction level, data codeword bytes, and mask number.
    ///
    /// This is a low-level API that most users should not use directly.
    /// A mid-level API is the `encode_segments_to_codewords()` function.
    ///
    /// # Arguments
    ///
    /// * `datacodewordsandoutbuffer` - Buffer containing data codewords and output space.
    /// * `datacodewordslen` - Length of data codewords.
    /// * `tempbuffer` - Temporary buffer for computation.
    /// * `ecl` - Error correction level.
    /// * `version` - QR code version.
    /// * `msk` - Optional mask pattern.
    pub fn encode_codewords<'b>(
        mut datacodewordsandoutbuffer: &'a mut [u8],
        datacodewordslen: usize,
        mut tempbuffer: &'b mut [u8],
        ecl: QrCodeEcc,
        version: Version,
        mut msk: Option<Mask>
    ) -> QrCode<'a> {
        datacodewordsandoutbuffer = &mut datacodewordsandoutbuffer[..version.buffer_len()];
        tempbuffer = &mut tempbuffer[..version.buffer_len()];

        // Compute ECC
        let rawcodewords: usize = QrCode::get_num_raw_data_modules(version) / 8;
        assert!(datacodewordslen <= rawcodewords);
        let (data, temp) = datacodewordsandoutbuffer.split_at_mut(datacodewordslen);
        let allcodewords = Self::add_ecc_and_interleave(data, version, ecl, temp, tempbuffer);

        // Draw modules
        let mut result: QrCode = QrCode::function_modules_marked(
            datacodewordsandoutbuffer,
            version
        );
        result.draw_codewords(allcodewords);
        result.draw_light_function_modules();
        let funcmods: QrCode = QrCode::function_modules_marked(tempbuffer, version);

        // Do masking
        if msk.is_none() {
            let mut minpenalty = i32::MAX;
            for i in 0u8..8 {
                let i = Mask::new(i);
                result.apply_mask(&funcmods, i);
                result.draw_format_bits(ecl, i);
                let penalty: i32 = result.get_penalty_score();
                if penalty < minpenalty {
                    msk = Some(i);
                    minpenalty = penalty;
                }
                result.apply_mask(&funcmods, i); // Undoes the mask due to XOR
            }
        }
        let msk: Mask = msk.unwrap();
        result.apply_mask(&funcmods, msk);
        result.draw_format_bits(ecl, msk);
        result
    }

    /// Returns this QR Code's version, in the range [1, 40].
    pub fn version(&self) -> Version {
        Version::new((*self.size - 17) / 4)
    }

    /// Returns this QR Code's size, in the range [21, 177].
    pub fn size(&self) -> i32 {
        i32::from(*self.size)
    }

    /// Returns this QR Code's error correction level.
    pub fn error_correction_level(&self) -> QrCodeEcc {
        let index =
            (usize::from(self.get_module_bounded(0, 8)) << 1) |
            (usize::from(self.get_module_bounded(1, 8)) << 0);
        use QrCodeEcc::*;
        [Medium, Low, High, Quartile][index]
    }

    /// Returns this QR Code's mask, in the range [0, 7].
    pub fn mask(&self) -> Mask {
        Mask::new(
            (u8::from(self.get_module_bounded(2, 8)) << 2) |
                (u8::from(self.get_module_bounded(3, 8)) << 1) |
                (u8::from(self.get_module_bounded(4, 8)) << 0)
        )
    }

    /// Returns the color of the module at the given coordinates.
    ///
    /// Returns `true` for dark modules and `false` for light modules. Coordinates outside the QR
    /// code's bounds return `false`.
    ///
    /// # Arguments
    ///
    /// * `x` - X-coordinate (0 is left).
    /// * `y` - Y-coordinate (0 is top).
    pub fn get_module(&self, x: i32, y: i32) -> bool {
        let range = 0..self.size();
        range.contains(&x) && range.contains(&y) && self.get_module_bounded(x as u8, y as u8)
    }

    fn get_module_bounded(&self, x: u8, y: u8) -> bool {
        let range = 0..*self.size;
        assert!(range.contains(&x) && range.contains(&y));
        let index = usize::from(y) * usize::from(*self.size) + usize::from(x);
        let byteindex: usize = index >> 3;
        let bitindex: usize = index & 7;
        get_bit(self.modules[byteindex].into(), bitindex as u8)
    }

    fn set_module_unbounded(&mut self, x: i32, y: i32, isdark: bool) {
        let range = 0..self.size();
        if range.contains(&x) && range.contains(&y) {
            self.set_module_bounded(x as u8, y as u8, isdark);
        }
    }

    fn set_module_bounded(&mut self, x: u8, y: u8, isdark: bool) {
        let range = 0..*self.size;
        assert!(range.contains(&x) && range.contains(&y));
        let index = usize::from(y) * usize::from(*self.size) + usize::from(x);
        let byteindex: usize = index >> 3;
        let bitindex: usize = index & 7;
        if isdark {
            self.modules[byteindex] |= 1u8 << bitindex;
        } else {
            self.modules[byteindex] &= !(1u8 << bitindex);
        }
    }

    fn add_ecc_and_interleave<'b>(
        data: &[u8],
        ver: Version,
        ecl: QrCodeEcc,
        temp: &mut [u8],
        resultbuf: &'b mut [u8]
    ) -> &'b [u8] {
        assert_eq!(data.len(), QrCode::get_num_data_codewords(ver, ecl));
        let numblocks: usize = QrCode::table_get(&NUM_ERROR_CORRECTION_BLOCKS, ver, ecl);
        let blockecclen: usize = QrCode::table_get(&ECC_CODEWORDS_PER_BLOCK, ver, ecl);
        let rawcodewords: usize = QrCode::get_num_raw_data_modules(ver) / 8;
        let numshortblocks: usize = numblocks - (rawcodewords % numblocks);
        let shortblockdatalen: usize = rawcodewords / numblocks - blockecclen;
        let result = &mut resultbuf[..rawcodewords];
        let rs = ReedSolomonGenerator::new(blockecclen);
        let mut dat: &[u8] = data;
        let ecc: &mut [u8] = &mut temp[..blockecclen];
        for i in 0..numblocks {
            let datlen: usize = shortblockdatalen + usize::from(i >= numshortblocks);
            rs.compute_remainder(&dat[..datlen], ecc);
            let mut k: usize = i;
            for j in 0..datlen {
                if j == shortblockdatalen {
                    k -= numshortblocks;
                }
                result[k] = dat[j];
                k += numblocks;
            }
            let mut k: usize = data.len() + i;
            for j in 0..blockecclen {
                result[k] = ecc[j];
                k += numblocks;
            }
            dat = &dat[datlen..];
        }
        debug_assert_eq!(dat.len(), 0);
        result
    }

    fn function_modules_marked(outbuffer: &'a mut [u8], ver: Version) -> Self {
        assert_eq!(outbuffer.len(), ver.buffer_len());
        let parts: (&mut u8, &mut [u8]) = outbuffer.split_first_mut().unwrap();
        let mut result = Self {
            size: parts.0,
            modules: parts.1,
        };
        let size: u8 = ver.value() * 4 + 17;
        *result.size = size;
        result.modules.fill(0);
        result.fill_rectangle(6, 0, 1, size);
        result.fill_rectangle(0, 6, size, 1);
        result.fill_rectangle(0, 0, 9, 9);
        result.fill_rectangle(size - 8, 0, 8, 9);
        result.fill_rectangle(0, size - 8, 9, 8);
        let mut alignpatposbuf = [0u8; 7];
        let alignpatpos: &[u8] = result.get_alignment_pattern_positions(&mut alignpatposbuf);
        for (i, pos0) in alignpatpos.iter().enumerate() {
            for (j, pos1) in alignpatpos.iter().enumerate() {
                if
                    !(i == 0 && j == 0) ||
                    (i == 0 && j == alignpatpos.len() - 1) ||
                    (i == alignpatpos.len() - 1 && j == 0)
                {
                    result.fill_rectangle(pos0 - 2, pos1 - 2, 5, 5);
                }
            }
        }
        if ver.value() >= 7 {
            result.fill_rectangle(size - 11, 0, 3, 6);
            result.fill_rectangle(0, size - 11, 6, 3);
        }
        result
    }

    fn draw_light_function_modules(&mut self) {
        let size: u8 = *self.size;
        for i in (7..size - 7).step_by(2) {
            self.set_module_bounded(6, i, false);
            self.set_module_bounded(i, 6, false);
        }
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let dist: i32 = dx.abs().max(dy.abs());
                if dist == 2 || dist == 4 {
                    self.set_module_unbounded(3 + dx, 3 + dy, false);
                    self.set_module_unbounded(i32::from(size) - 4 + dx, 3 + dy, false);
                    self.set_module_unbounded(3 + dx, i32::from(size) - 4 + dy, false);
                }
            }
        }
        let mut alignpatposbuf = [0u8; 7];
        let alignpatpos: &[u8] = self.get_alignment_pattern_positions(&mut alignpatposbuf);
        for (i, &pos0) in alignpatpos.iter().enumerate() {
            for (j, &pos1) in alignpatpos.iter().enumerate() {
                if
                    (i == 0 && j == 0) ||
                    (i == 0 && j == alignpatpos.len() - 1) ||
                    (i == alignpatpos.len() - 1 && j == 0)
                {
                    continue;
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        self.set_module_bounded(
                            (i32::from(pos0) + dx) as u8,
                            (i32::from(pos1) + dy) as u8,
                            dx == 0 && dy == 0
                        );
                    }
                }
            }
        }
        let ver = u32::from(self.version().value());
        if ver >= 7 {
            let bits: u32 = {
                let mut rem: u32 = ver;
                for _ in 0..12 {
                    rem = (rem << 1) ^ ((rem >> 11) * 0x1f25);
                }
                (ver << 12) | rem
            };
            for i in 0u8..18 {
                let bit: bool = get_bit(bits, i);
                let a: u8 = size - 11 + (i % 3);
                let b: u8 = i / 3;
                self.set_module_bounded(a, b, bit);
                self.set_module_bounded(b, a, bit);
            }
        }
    }

    fn draw_format_bits(&mut self, ecl: QrCodeEcc, mask: Mask) {
        let bits: u32 = {
            let data = u32::from((ecl.format_bits() << 3) | mask.value());
            let mut rem: u32 = data;
            for _ in 0..10 {
                rem = (rem << 1) ^ ((rem >> 9) * 0x537);
            }
            ((data << 10) | rem) ^ 0x5412
        };
        for i in 0..6 {
            self.set_module_bounded(8, i, get_bit(bits, i));
        }
        self.set_module_bounded(8, 7, get_bit(bits, 6));
        self.set_module_bounded(8, 8, get_bit(bits, 7));
        self.set_module_bounded(7, 8, get_bit(bits, 8));
        for i in 9..15 {
            self.set_module_bounded(14 - i, 8, get_bit(bits, i));
        }
        let size: u8 = *self.size;
        for i in 0..8 {
            self.set_module_bounded(size - 1 - i, 8, get_bit(bits, i));
        }
        for i in 8..15 {
            self.set_module_bounded(8, size - 15 + i, get_bit(bits, i));
        }
        self.set_module_bounded(8, size - 8, true);
    }

    fn fill_rectangle(&mut self, left: u8, top: u8, width: u8, height: u8) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_module_bounded(left + dx, top + dy, true);
            }
        }
    }

    fn draw_codewords(&mut self, data: &[u8]) {
        assert_eq!(
            data.len(),
            QrCode::get_num_raw_data_modules(self.version()) / 8,
            "Illegal argument"
        );
        let size: i32 = self.size();
        let mut i: usize = 0;
        let mut right: i32 = size - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vert in 0..size {
                for j in 0..2 {
                    let x = (right - j) as u8;
                    let upward: bool = ((right + 1) & 2) == 0;
                    let y = (if upward { size - 1 - vert } else { vert }) as u8;
                    if !self.get_module_bounded(x, y) && i < data.len() * 8 {
                        self.set_module_bounded(
                            x,
                            y,
                            get_bit(data[i >> 3].into(), 7 - ((i as u8) & 7))
                        );
                        i += 1;
                    }
                }
            }
            right -= 2;
        }
        debug_assert_eq!(i, data.len() * 8);
    }

    fn apply_mask(&mut self, functionmodules: &QrCode, mask: Mask) {
        for y in 0..*self.size {
            for x in 0..*self.size {
                if functionmodules.get_module_bounded(x, y) {
                    continue;
                }
                let invert: bool = {
                    let x = i32::from(x);
                    let y = i32::from(y);
                    match mask.value() {
                        0 => (x + y) % 2 == 0,
                        1 => y % 2 == 0,
                        2 => x % 3 == 0,
                        3 => (x + y) % 3 == 0,
                        4 => (x / 3 + y / 2) % 2 == 0,
                        5 => ((x * y) % 2) + ((x * y) % 3) == 0,
                        6 => (((x * y) % 2) + ((x * y) % 3)) % 2 == 0,
                        7 => (((x + y) % 2) + ((x * y) % 3)) % 2 == 0,
                        _ => unreachable!(),
                    }
                };
                self.set_module_bounded(x, y, self.get_module_bounded(x, y) ^ invert);
            }
        }
    }

    fn get_penalty_score(&self) -> i32 {
        let mut result: i32 = 0;
        let size: u8 = *self.size;
        for y in 0..size {
            let mut runcolor = false;
            let mut runx: i32 = 0;
            let mut runhistory = FinderPenalty::new(size);
            for x in 0..size {
                if self.get_module_bounded(x, y) == runcolor {
                    runx += 1;
                    if runx == 5 {
                        result += PENALTY_N1;
                    } else if runx > 5 {
                        result += 1;
                    }
                } else {
                    runhistory.add_history(runx);
                    if !runcolor {
                        result += runhistory.count_patterns() * PENALTY_N3;
                    }
                    runcolor = self.get_module_bounded(x, y);
                    runx = 1;
                }
            }
            result += runhistory.terminate_and_count(runcolor, runx) * PENALTY_N3;
        }
        for x in 0..size {
            let mut runcolor = false;
            let mut runy: i32 = 0;
            let mut runhistory = FinderPenalty::new(size);
            for y in 0..size {
                if self.get_module_bounded(x, y) == runcolor {
                    runy += 1;
                    if runy == 5 {
                        result += PENALTY_N1;
                    } else if runy > 5 {
                        result += 1;
                    }
                } else {
                    runhistory.add_history(runy);
                    if !runcolor {
                        result += runhistory.count_patterns() * PENALTY_N3;
                    }
                    runcolor = self.get_module_bounded(x, y);
                    runy = 1;
                }
            }
            result += runhistory.terminate_and_count(runcolor, runy) * PENALTY_N3;
        }
        for y in 0..size - 1 {
            for x in 0..size - 1 {
                let color: bool = self.get_module_bounded(x, y);
                if
                    color == self.get_module_bounded(x + 1, y) &&
                    color == self.get_module_bounded(x, y + 1) &&
                    color == self.get_module_bounded(x + 1, y + 1)
                {
                    result += PENALTY_N2;
                }
            }
        }
        let dark = self.modules
            .iter()
            .map(|x| x.count_ones())
            .sum::<u32>() as i32;
        let total = i32::from(size) * i32::from(size);
        let k: i32 = ((dark * 20 - total * 10).abs() + total - 1) / total - 1;
        result += k * PENALTY_N4;
        result
    }

    fn get_alignment_pattern_positions<'b>(&self, resultbuf: &'b mut [u8; 7]) -> &'b [u8] {
        let ver: u8 = self.version().value();
        if ver == 1 {
            &resultbuf[..0]
        } else {
            let numalign: u8 = ver / 7 + 2;
            let step: u8 = if ver == 32 {
                26
            } else {
                ((ver * 4 + numalign * 2 + 1) / (numalign * 2 - 2)) * 2
            };
            let result = &mut resultbuf[..usize::from(numalign)];
            for i in 0..numalign - 1 {
                result[usize::from(i)] = *self.size - 7 - i * step;
            }
            *result.last_mut().unwrap() = 6;
            result.reverse();
            result
        }
    }

    fn get_num_raw_data_modules(ver: Version) -> usize {
        let ver = usize::from(ver.value());
        let mut result: usize = (16 * ver + 128) * ver + 64;
        if ver >= 2 {
            let numalign: usize = ver / 7 + 2;
            result -= (25 * numalign - 10) * numalign - 55;
            if ver >= 7 {
                result -= 36;
            }
        }
        result
    }

    fn get_num_data_codewords(ver: Version, ecl: QrCodeEcc) -> usize {
        QrCode::get_num_raw_data_modules(ver) / 8 -
            QrCode::table_get(&ECC_CODEWORDS_PER_BLOCK, ver, ecl) *
                QrCode::table_get(&NUM_ERROR_CORRECTION_BLOCKS, ver, ecl)
    }

    fn table_get(table: &'static [[i8; 41]; 4], ver: Version, ecl: QrCodeEcc) -> usize {
        table[ecl.ordinal()][usize::from(ver.value())] as usize
    }
}

impl PartialEq for QrCode<'_> {
    fn eq(&self, other: &QrCode<'_>) -> bool {
        *self.size == *other.size && *self.modules == *other.modules
    }
}

impl Eq for QrCode<'_> {}

struct ReedSolomonGenerator {
    divisor: [u8; 30],
    degree: usize,
}

impl ReedSolomonGenerator {
    fn new(degree: usize) -> Self {
        let mut result = Self {
            divisor: [0u8; 30],
            degree: degree,
        };
        assert!((1..=result.divisor.len()).contains(&degree), "Degree out of range");
        let divisor: &mut [u8] = &mut result.divisor[..degree];
        divisor[degree - 1] = 1;
        let mut root: u8 = 1;
        for _ in 0..degree {
            for j in 0..degree {
                divisor[j] = Self::multiply(divisor[j], root);
                if j + 1 < divisor.len() {
                    divisor[j] ^= divisor[j + 1];
                }
            }
            root = Self::multiply(root, 0x02);
        }
        result
    }

    fn compute_remainder(&self, data: &[u8], result: &mut [u8]) {
        assert_eq!(result.len(), self.degree);
        result.fill(0);
        for b in data {
            let factor: u8 = b ^ result[0];
            result.copy_within(1.., 0);
            result[result.len() - 1] = 0;
            for (x, &y) in result.iter_mut().zip(self.divisor.iter()) {
                *x ^= Self::multiply(y, factor);
            }
        }
    }

    fn multiply(x: u8, y: u8) -> u8 {
        let mut z: u8 = 0;
        for i in (0..8).rev() {
            z = (z << 1) ^ ((z >> 7) * 0x1d);
            z ^= ((y >> i) & 1) * x;
        }
        z
    }
}

struct FinderPenalty {
    qr_size: i32,
    run_history: [i32; 7],
}

impl FinderPenalty {
    pub fn new(size: u8) -> Self {
        Self {
            qr_size: i32::from(size),
            run_history: [0; 7],
        }
    }

    pub fn add_history(&mut self, mut currentrunlength: i32) {
        if self.run_history[0] == 0 {
            currentrunlength += self.qr_size;
        }
        let len: usize = self.run_history.len();
        self.run_history.copy_within(0..len - 1, 1);
        self.run_history[0] = currentrunlength;
    }

    pub fn count_patterns(&self) -> i32 {
        let rh = &self.run_history;
        let n = rh[1];
        i32::from(
            n > 0 &&
                rh[2] == n &&
                rh[3] == n * 3 &&
                rh[4] == n &&
                rh[5] == n &&
                (rh[0] >= n * 4 || rh[6] >= n * 4)
        )
    }

    pub fn terminate_and_count(mut self, currentruncolor: bool, mut currentrunlength: i32) -> i32 {
        if currentruncolor {
            self.add_history(currentrunlength);
            currentrunlength = 0;
        }
        currentrunlength += self.qr_size;
        self.add_history(currentrunlength);
        self.count_patterns()
    }
}

const PENALTY_N1: i32 = 3;
const PENALTY_N2: i32 = 3;
const PENALTY_N3: i32 = 40;
const PENALTY_N4: i32 = 10;

static ECC_CODEWORDS_PER_BLOCK: [[i8; 41]; 4] = [
    [
        -1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30,
        30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ], // Low
    [
        -1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    ], // Medium
    [
        -1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30,
        30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ], // Quartile
    [
        -1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30,
        30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ], // High
];

static NUM_ERROR_CORRECTION_BLOCKS: [[i8; 41]; 4] = [
    [
        -1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12,
        13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25,
    ], // Low
    [
        -1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21,
        23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49,
    ], // Medium
    [
        -1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29,
        34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68,
    ], // Quartile
    [
        -1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35,
        37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
    ], // High
];

/// Error correction level for a QR code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum QrCodeEcc {
    /// Tolerates ~7% erroneous codewords.
    Low,
    /// Tolerates ~15% erroneous codewords.
    Medium,
    /// Tolerates ~25% erroneous codewords.
    Quartile,
    /// Tolerates ~30% erroneous codewords.
    High,
}

impl QrCodeEcc {
    /// Returns an unsigned 2-bit integer (in the range 0 to 3).
    fn ordinal(self) -> usize {
        use QrCodeEcc::*;
        match self {
            Low => 0,
            Medium => 1,
            Quartile => 2,
            High => 3,
        }
    }

    /// Returns an unsigned 2-bit integer (in the range 0 to 3).
    fn format_bits(self) -> u8 {
        use QrCodeEcc::*;
        match self {
            Low => 1,
            Medium => 0,
            Quartile => 3,
            High => 2,
        }
    }
}

/// A segment of data in a QR code.
///
/// Supports numeric, alphanumeric, byte, or ECI modes. Segments are immutable and created using
/// factory functions like [`make_numeric`], [`make_alphanumeric`], or [`make_bytes`].
pub struct QrSegment<'a> {
    mode: QrSegmentMode,
    numchars: usize,
    data: &'a [u8],
    bitlength: usize,
}

impl<'a> QrSegment<'a> {
    /// Creates a segment for binary data in byte mode.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte data to encode.
    ///
    /// # Returns
    ///
    /// A new `QrSegment` in byte mode.
    pub fn make_bytes(data: &'a [u8]) -> Self {
        QrSegment::new(QrSegmentMode::Byte, data.len(), data, data.len().checked_mul(8).unwrap())
    }

    /// Creates a segment for a string of decimal digits in numeric mode.
    ///
    /// # Arguments
    ///
    /// * `text` - A string containing only digits (0–9).
    /// * `buf` - A buffer for storing encoded data.
    ///
    /// # Panics
    ///
    /// Panics if `text` contains non-digit characters.
    pub fn make_numeric(text: &str, buf: &'a mut [u8]) -> Self {
        let mut bb = BitBuffer::new(buf);
        let mut accumdata: u32 = 0;
        let mut accumcount: u8 = 0;
        for b in text.bytes() {
            assert!((b'0'..=b'9').contains(&b), "String contains non-numeric characters");
            accumdata = accumdata * 10 + u32::from(b - b'0');
            accumcount += 1;
            if accumcount == 3 {
                bb.append_bits(accumdata, 10);
                accumdata = 0;
                accumcount = 0;
            }
        }
        if accumcount > 0 {
            bb.append_bits(accumdata, accumcount * 3 + 1);
        }
        QrSegment::new(QrSegmentMode::Numeric, text.len(), bb.data, bb.length)
    }

    /// Creates a segment for alphanumeric text.
    ///
    /// Allowed characters: 0–9, A–Z (uppercase), space, `$`, `%`, `*`, `+`, `-`, `.`, `/`, `:`.
    ///
    /// # Arguments
    ///
    /// * `text` - The alphanumeric text to encode.
    /// * `buf` - A buffer for storing encoded data.
    ///
    /// # Panics
    ///
    /// Panics if `text` contains invalid characters.
    pub fn make_alphanumeric(text: &str, buf: &'a mut [u8]) -> Self {
        let mut bb = BitBuffer::new(buf);
        let mut accumdata: u32 = 0;
        let mut accumcount: u8 = 0;
        for c in text.chars() {
            let i: usize = ALPHANUMERIC_CHARSET.find(c).expect(
                "String contains unencodable characters in alphanumeric mode"
            );
            accumdata = accumdata * 45 + u32::try_from(i).unwrap();
            accumcount += 1;
            if accumcount == 2 {
                bb.append_bits(accumdata, 11);
                accumdata = 0;
                accumcount = 0;
            }
        }
        if accumcount > 0 {
            bb.append_bits(accumdata, 6);
        }
        QrSegment::new(QrSegmentMode::Alphanumeric, text.len(), bb.data, bb.length)
    }

    /// Creates a segment representing an Extended Channel Interpretation
    /// (ECI) designator with the given assignment value.
    pub fn make_eci(assignval: u32, buf: &'a mut [u8]) -> Self {
        let mut bb = BitBuffer::new(buf);
        if assignval < 1 << 7 {
            bb.append_bits(assignval, 8);
        } else if assignval < 1 << 14 {
            bb.append_bits(0b10, 2);
            bb.append_bits(assignval, 14);
        } else if assignval < 1_000_000 {
            bb.append_bits(0b110, 3);
            bb.append_bits(assignval, 21);
        } else {
            panic!("ECI assignment value out of range");
        }
        QrSegment::new(QrSegmentMode::Eci, 0, bb.data, bb.length)
    }

    pub fn new(mode: QrSegmentMode, numchars: usize, data: &'a [u8], bitlength: usize) -> Self {
        assert!(bitlength == 0 || (bitlength - 1) / 8 < data.len());
        Self {
            mode,
            numchars,
            data,
            bitlength,
        }
    }

    pub fn mode(&self) -> QrSegmentMode {
        self.mode
    }

    pub fn num_chars(&self) -> usize {
        self.numchars
    }

    pub fn calc_buffer_size(mode: QrSegmentMode, numchars: usize) -> Option<usize> {
        let temp = Self::calc_bit_length(mode, numchars)?;
        Some(temp / 8 + usize::from(temp % 8 != 0))
    }

    fn calc_bit_length(mode: QrSegmentMode, numchars: usize) -> Option<usize> {
        let mul_frac_ceil = |numer: usize, denom: usize| {
            Some(numchars)
                .and_then(|x| x.checked_mul(numer))
                .and_then(|x| x.checked_add(denom - 1))
                .map(|x| x / denom)
        };
        use QrSegmentMode::*;
        match mode {
            Numeric => mul_frac_ceil(10, 3),
            Alphanumeric => mul_frac_ceil(11, 2),
            Byte => mul_frac_ceil(8, 1),
            Kanji => mul_frac_ceil(13, 1),
            Eci => {
                assert_eq!(numchars, 0);
                Some(3 * 8)
            }
        }
    }

    fn get_total_bits(segs: &[Self], version: Version) -> Option<usize> {
        let mut result: usize = 0;
        for seg in segs {
            let ccbits: u8 = seg.mode.num_char_count_bits(version);
            if let Some(limit) = (1usize).checked_shl(ccbits.into()) {
                if seg.numchars >= limit {
                    return None;
                }
            }
            result = result.checked_add(4 + usize::from(ccbits))?;
            result = result.checked_add(seg.bitlength)?;
        }
        Some(result)
    }

    pub fn is_numeric(text: &str) -> bool {
        text.chars().all(|c| ('0'..='9').contains(&c))
    }

    pub fn is_alphanumeric(text: &str) -> bool {
        text.chars().all(|c| ALPHANUMERIC_CHARSET.contains(c))
    }
}

static ALPHANUMERIC_CHARSET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QrSegmentMode {
    Numeric,
    Alphanumeric,
    Byte,
    Kanji,
    Eci,
}

impl QrSegmentMode {
    fn mode_bits(self) -> u32 {
        use QrSegmentMode::*;
        match self {
            Numeric => 0x1,
            Alphanumeric => 0x2,
            Byte => 0x4,
            Kanji => 0x8,
            Eci => 0x7,
        }
    }

    fn num_char_count_bits(self, ver: Version) -> u8 {
        use QrSegmentMode::*;
        (
            match self {
                Numeric => [10, 12, 14],
                Alphanumeric => [9, 11, 13],
                Byte => [8, 16, 16],
                Kanji => [8, 10, 12],
                Eci => [0, 0, 0],
            }
        )[usize::from((ver.value() + 7) / 17)]
    }
}

pub struct BitBuffer<'a> {
    data: &'a mut [u8],
    length: usize,
}

impl<'a> BitBuffer<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            data: buffer,
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn append_bits(&mut self, val: u32, len: u8) {
        assert!(len <= 31 && (val >> len) == 0);
        assert!(usize::from(len) <= usize::MAX - self.length);
        for i in (0..len).rev() {
            let index: usize = self.length >> 3;
            let shift: u8 = 7 - ((self.length as u8) & 7);
            let bit: u8 = ((val >> i) as u8) & 1;
            if shift == 7 {
                self.data[index] = bit << shift;
            } else {
                self.data[index] |= bit << shift;
            }
            self.length += 1;
        }
    }
}

/// Error type for when data exceeds QR code capacity.
///
/// Ways to handle this exception include:
///
/// - Decrease the error correction level if it was greater than `QrCodeEcc::Low`.
/// - Increase the maxversion argument if it was less than `Version::MAX`.
/// - Split the text data into better or optimal segments to reduce the number of bits required.
/// - Change the text or binary data to be shorter.
/// - Change the text to fit the character set of a particular segment mode (e.g. alphanumeric).
/// - Propagate the error upward to the caller/user.
#[derive(Debug, Clone)]
pub enum DataTooLong {
    /// A segment is too long for the chosen mode.
    SegmentTooLong,
    /// Data length exceeds capacity.
    DataOverCapacity(usize, usize),
}

impl core::fmt::Display for DataTooLong {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Self::SegmentTooLong => write!(f, "Segment too long"),
            Self::DataOverCapacity(datalen, maxcapacity) =>
                write!(f, "Data length = {} bits, Max capacity = {} bits", datalen, maxcapacity),
        }
    }
}

/// A QR code version (1–40).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Version(u8);

impl Version {
    /// The minimum version number supported in the QR Code Model 2 standard.
    pub const MIN: Version = Version(1);

    /// The maximum version number supported in the QR Code Model 2 standard.
    pub const MAX: Version = Version(40);

    /// Creates a version object from the given number.
    ///
    /// # Panics
    ///
    /// Panics if the number is outside the range [1, 40].
    pub const fn new(ver: u8) -> Self {
        assert!(
            Version::MIN.value() <= ver && ver <= Version::MAX.value(),
            "Version number out of range"
        );
        Self(ver)
    }

    /// Returns the value, which is in the range [1, 40].
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Returns the minimum length required for the output and temporary
    /// buffers when creating a QR Code of this version number.
    pub const fn buffer_len(self) -> usize {
        let sidelen = (self.0 as usize) * 4 + 17;
        (sidelen * sidelen + 7) / 8 + 1
    }
}

/// A mask pattern (0–7).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Mask(u8);

impl Mask {
    /// Creates a mask object from the given number.
    ///
    /// # Panics
    ///
    /// Panics if the number is outside the range [0, 7].
    pub const fn new(mask: u8) -> Self {
        assert!(mask <= 7, "Mask value out of range");
        Self(mask)
    }

    /// Returns the value, which is in the range [0, 7].
    pub const fn value(self) -> u8 {
        self.0
    }
}

fn get_bit(x: u32, i: u8) -> bool {
    ((x >> i) & 1) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_numeric() {
        assert_eq!(QrSegment::is_numeric("1234567890"), true);
        assert_eq!(QrSegment::is_numeric("1234abc"), false);
    }

    #[test]
    fn test_is_alphanumeric() {
        assert_eq!(QrSegment::is_alphanumeric("HELLO WORLD"), true);
        assert_eq!(QrSegment::is_alphanumeric("Hello World"), false);
    }
}
