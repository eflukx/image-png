//! Common types shared between the encoder and decoder
use crate::filter;

use std::{convert::TryFrom, fmt};

/// Describes the layout of samples in a pixel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorType {
    Grayscale = 0,
    Rgb = 2,
    Indexed = 3,
    GrayscaleAlpha = 4,
    Rgba = 6,
}

impl ColorType {
    /// Returns the number of samples used per pixel of `ColorType`
    pub fn samples(self) -> usize {
        self.samples_u8().into()
    }

    pub(crate) fn samples_u8(self) -> u8 {
        use self::ColorType::*;
        match self {
            Grayscale | Indexed => 1,
            Rgb => 3,
            GrayscaleAlpha => 2,
            Rgba => 4,
        }
    }

    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub fn from_u8(n: u8) -> Option<ColorType> {
        match n {
            0 => Some(ColorType::Grayscale),
            2 => Some(ColorType::Rgb),
            3 => Some(ColorType::Indexed),
            4 => Some(ColorType::GrayscaleAlpha),
            6 => Some(ColorType::Rgba),
            _ => None,
        }
    }

    pub(crate) fn checked_raw_row_length(self, depth: BitDepth, width: u32) -> Option<usize> {
        // No overflow can occur in 64 bits, we multiply 32-bit with 5 more bits.
        let bits = u64::from(width) * u64::from(self.samples_u8()) * u64::from(depth.into_u8());
        TryFrom::try_from(1 + (bits + 7) / 8).ok()
    }

    pub(crate) fn raw_row_length_from_width(self, depth: BitDepth, width: u32) -> usize {
        let samples = width as usize * self.samples();
        1 + match depth {
            BitDepth::Sixteen => samples * 2,
            BitDepth::Eight => samples,
            subbyte => {
                let samples_per_byte = 8 / subbyte as usize;
                let whole = samples / samples_per_byte;
                let fract = usize::from(samples % samples_per_byte > 0);
                whole + fract
            }
        }
    }

    pub(crate) fn is_combination_invalid(self, bit_depth: BitDepth) -> bool {
        // Section 11.2.2 of the PNG standard disallows several combinations
        // of bit depth and color type
        ((bit_depth == BitDepth::One || bit_depth == BitDepth::Two || bit_depth == BitDepth::Four)
            && (self == ColorType::Rgb
                || self == ColorType::GrayscaleAlpha
                || self == ColorType::Rgba))
            || (bit_depth == BitDepth::Sixteen && self == ColorType::Indexed)
    }
}

/// Bit depth of the png file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BitDepth {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

/// Internal count of bytes per pixel.
/// This is used for filtering which never uses sub-byte units. This essentially reduces the number
/// of possible byte chunk lengths to a very small set of values appropriate to be defined as an
/// enum.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum BytesPerPixel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Six = 6,
    Eight = 8,
}

impl BitDepth {
    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub fn from_u8(n: u8) -> Option<BitDepth> {
        match n {
            1 => Some(BitDepth::One),
            2 => Some(BitDepth::Two),
            4 => Some(BitDepth::Four),
            8 => Some(BitDepth::Eight),
            16 => Some(BitDepth::Sixteen),
            _ => None,
        }
    }

    pub(crate) fn into_u8(self) -> u8 {
        self as u8
    }
}

/// Pixel dimensions information
#[derive(Clone, Copy, Debug)]
pub struct PixelDimensions {
    /// Pixels per unit, X axis
    pub xppu: u32,
    /// Pixels per unit, Y axis
    pub yppu: u32,
    /// Either *Meter* or *Unspecified*
    pub unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
/// Physical unit of the pixel dimensions
pub enum Unit {
    Unspecified = 0,
    Meter = 1,
}

impl Unit {
    /// u8 -> Self. Temporary solution until Rust provides a canonical one.
    pub fn from_u8(n: u8) -> Option<Unit> {
        match n {
            0 => Some(Unit::Unspecified),
            1 => Some(Unit::Meter),
            _ => None,
        }
    }
}

/// How to reset buffer of an animated png (APNG) at the end of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DisposeOp {
    /// Leave the buffer unchanged.
    None = 0,
    /// Clear buffer with the background color.
    Background = 1,
    /// Reset the buffer to the state before the current frame.
    Previous = 2,
}

impl DisposeOp {
    /// u8 -> Self. Using enum_primitive or transmute is probably the right thing but this will do for now.
    pub fn from_u8(n: u8) -> Option<DisposeOp> {
        match n {
            0 => Some(DisposeOp::None),
            1 => Some(DisposeOp::Background),
            2 => Some(DisposeOp::Previous),
            _ => None,
        }
    }
}

impl fmt::Display for DisposeOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            DisposeOp::None => "DISPOSE_OP_NONE",
            DisposeOp::Background => "DISPOSE_OP_BACKGROUND",
            DisposeOp::Previous => "DISPOSE_OP_PREVIOUS",
        };
        write!(f, "{}", name)
    }
}

/// How pixels are written into the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BlendOp {
    /// Pixels overwrite the value at their position.
    Source = 0,
    /// The new pixels are blended into the current state based on alpha.
    Over = 1,
}

impl BlendOp {
    /// u8 -> Self. Using enum_primitive or transmute is probably the right thing but this will do for now.
    pub fn from_u8(n: u8) -> Option<BlendOp> {
        match n {
            0 => Some(BlendOp::Source),
            1 => Some(BlendOp::Over),
            _ => None,
        }
    }
}

impl fmt::Display for BlendOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            BlendOp::Source => "BLEND_OP_SOURCE",
            BlendOp::Over => "BLEND_OP_OVER",
        };
        write!(f, "{}", name)
    }
}

/// Frame control information
#[derive(Clone, Copy, Debug)]
pub struct FrameControl {
    /// Sequence number of the animation chunk, starting from 0
    pub sequence_number: u32,
    /// Width of the following frame
    pub width: u32,
    /// Height of the following frame
    pub height: u32,
    /// X position at which to render the following frame
    pub x_offset: u32,
    /// Y position at which to render the following frame
    pub y_offset: u32,
    /// Frame delay fraction numerator
    pub delay_num: u16,
    /// Frame delay fraction denominator
    pub delay_den: u16,
    /// Type of frame area disposal to be done after rendering this frame
    pub dispose_op: DisposeOp,
    /// Type of frame area rendering for this frame
    pub blend_op: BlendOp,
}

impl Default for FrameControl {
    fn default() -> FrameControl {
        FrameControl {
            sequence_number: 0,
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
            delay_num: 1,
            delay_den: 30,
            dispose_op: DisposeOp::None,
            blend_op: BlendOp::Source,
        }
    }
}

impl FrameControl {
    pub fn set_seq_num(&mut self, s: u32) {
        self.sequence_number = s;
    }

    pub fn inc_seq_num(&mut self, i: u32) {
        self.sequence_number += i;
    }
}

/// Animation control information
#[derive(Clone, Copy, Debug)]
pub struct AnimationControl {
    /// Number of frames
    pub num_frames: u32,
    /// Number of times to loop this APNG.  0 indicates infinite looping.
    pub num_plays: u32,
}

/// The type and strength of applied compression.
#[derive(Debug, Clone)]
pub enum Compression {
    /// Default level  
    Default,
    /// Fast minimal compression
    Fast,
    /// Higher compression level  
    ///
    /// Best in this context isn't actually the highest possible level
    /// the encoder can do, but is meant to emulate the `Best` setting in the `Flate2`
    /// library.
    Best,
    Huffman,
    Rle,
}

/// An unsigned integer scaled version of a floating point value,
/// equivalent to an integer quotient with fixed denominator (100_000)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScaledFloat(u32);

impl ScaledFloat {
    const SCALING: f32 = 100_000.0;

    /// Gets whether the value is within the clamped range of this type.
    pub fn in_range(value: f32) -> bool {
        value >= 0.0 && (value * Self::SCALING).floor() <= std::u32::MAX as f32
    }

    /// Gets whether the value can be exactly converted in round-trip.
    #[allow(clippy::float_cmp)] // Stupid tool, the exact float compare is _the entire point_.
    pub fn exact(value: f32) -> bool {
        let there = Self::forward(value);
        let back = Self::reverse(there);
        value == back
    }

    fn forward(value: f32) -> u32 {
        (value.max(0.0) * Self::SCALING).floor() as u32
    }

    fn reverse(encoded: u32) -> f32 {
        encoded as f32 / Self::SCALING
    }

    /// Slightly inaccurate scaling and quantization.
    /// Clamps the value into the representible range if it is negative of too large.
    pub fn new(value: f32) -> Self {
        Self {
            0: Self::forward(value),
        }
    }

    /// Fully accurate construction from a value scaled as per specification.
    pub fn from_scaled(val: u32) -> Self {
        Self { 0: val }
    }

    /// Get the accurate encoded value.
    pub fn into_scaled(self) -> u32 {
        self.0
    }

    /// Get the unscaled value as a floating point.
    pub fn into_value(self) -> f32 {
        Self::reverse(self.0) as f32
    }
}

/// Chromaticities of the color space primaries
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceChromaticities {
    pub white: (ScaledFloat, ScaledFloat),
    pub red: (ScaledFloat, ScaledFloat),
    pub green: (ScaledFloat, ScaledFloat),
    pub blue: (ScaledFloat, ScaledFloat),
}

impl SourceChromaticities {
    pub fn new(white: (f32, f32), red: (f32, f32), green: (f32, f32), blue: (f32, f32)) -> Self {
        SourceChromaticities {
            white: (ScaledFloat::new(white.0), ScaledFloat::new(white.1)),
            red: (ScaledFloat::new(red.0), ScaledFloat::new(red.1)),
            green: (ScaledFloat::new(green.0), ScaledFloat::new(green.1)),
            blue: (ScaledFloat::new(blue.0), ScaledFloat::new(blue.1)),
        }
    }
}

/// PNG info struct
#[derive(Clone, Debug)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub bit_depth: BitDepth,
    pub color_type: ColorType,
    pub interlaced: bool,
    pub trns: Option<Vec<u8>>,
    pub pixel_dims: Option<PixelDimensions>,
    /// Source system's gamma
    pub source_gamma: Option<ScaledFloat>,
    pub palette: Option<Vec<u8>>,
    pub frame_control: Option<FrameControl>,
    pub animation_control: Option<AnimationControl>,
    pub compression: Compression,
    pub filter: filter::FilterType,
    pub source_chromaticities: Option<SourceChromaticities>,
}

impl Default for Info {
    fn default() -> Info {
        Info {
            width: 0,
            height: 0,
            bit_depth: BitDepth::Eight,
            color_type: ColorType::Grayscale,
            interlaced: false,
            palette: None,
            trns: None,
            pixel_dims: None,
            source_gamma: None,
            frame_control: None,
            animation_control: None,
            // Default to `deflate::Compresion::Fast` and `filter::FilterType::Sub`
            // to maintain backward compatible output.
            compression: Compression::Fast,
            filter: filter::FilterType::Sub,
            source_chromaticities: None,
        }
    }
}

impl Info {
    /// Size of the image
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns true if the image is an APNG image.
    pub fn is_animated(&self) -> bool {
        self.frame_control.is_some() && self.animation_control.is_some()
    }

    /// Returns the frame control information of the image
    pub fn animation_control(&self) -> Option<&AnimationControl> {
        self.animation_control.as_ref()
    }

    /// Returns the frame control information of the current frame
    pub fn frame_control(&self) -> Option<&FrameControl> {
        self.frame_control.as_ref()
    }

    /// Returns the bits per pixel
    pub fn bits_per_pixel(&self) -> usize {
        self.color_type.samples() * self.bit_depth as usize
    }

    /// Returns the bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        // If adjusting this for expansion or other transformation passes, remember to keep the old
        // implementation for bpp_in_prediction, which is internal to the png specification.
        self.color_type.samples() * ((self.bit_depth as usize + 7) >> 3)
    }

    /// Return the number of bytes for this pixel used in prediction.
    ///
    /// Some filters use prediction, over the raw bytes of a scanline. Where a previous pixel is
    /// require for such forms the specification instead references previous bytes. That is, for
    /// a gray pixel of bit depth 2, the pixel used in prediction is actually 4 pixels prior. This
    /// has the consequence that the number of possible values is rather small. To make this fact
    /// more obvious in the type system and the optimizer we use an explicit enum here.
    pub(crate) fn bpp_in_prediction(&self) -> BytesPerPixel {
        match self.bytes_per_pixel() {
            1 => BytesPerPixel::One,
            2 => BytesPerPixel::Two,
            3 => BytesPerPixel::Three,
            4 => BytesPerPixel::Four,
            6 => BytesPerPixel::Six,   // Only rgb×16bit
            8 => BytesPerPixel::Eight, // Only rgba×16bit
            _ => unreachable!("Not a possible byte rounded pixel width"),
        }
    }

    /// Returns the number of bytes needed for one deinterlaced image
    pub fn raw_bytes(&self) -> usize {
        self.height as usize * self.raw_row_length()
    }

    /// Returns the number of bytes needed for one deinterlaced row
    pub fn raw_row_length(&self) -> usize {
        self.raw_row_length_from_width(self.width)
    }

    pub(crate) fn checked_raw_row_length(&self) -> Option<usize> {
        self.color_type
            .checked_raw_row_length(self.bit_depth, self.width)
    }

    /// Returns the number of bytes needed for one deinterlaced row of width `width`
    pub fn raw_row_length_from_width(&self, width: u32) -> usize {
        self.color_type
            .raw_row_length_from_width(self.bit_depth, width)
    }
}

impl BytesPerPixel {
    pub(crate) fn into_usize(self) -> usize {
        self as usize
    }
}

bitflags! {
    /// Output transformations
    pub struct Transformations: u32 {
        /// No transformation
        const IDENTITY            = 0x0000; // read and write */
        /// Strip 16-bit samples to 8 bits
        const STRIP_16            = 0x0001; // read only */
        /// Discard the alpha channel
        const STRIP_ALPHA         = 0x0002; // read only */
        /// Expand 1; 2 and 4-bit samples to bytes
        const PACKING             = 0x0004; // read and write */
        /// Change order of packed pixels to LSB first
        const PACKSWAP            = 0x0008; // read and write */
        /// Expand paletted images to RGB; expand grayscale images of
        /// less than 8-bit depth to 8-bit depth; and expand tRNS chunks
        /// to alpha channels.
        const EXPAND              = 0x0010; // read only */
        /// Invert monochrome images
        const INVERT_MONO         = 0x0020; // read and write */
        /// Normalize pixels to the sBIT depth
        const SHIFT               = 0x0040; // read and write */
        /// Flip RGB to BGR; RGBA to BGRA
        const BGR                 = 0x0080; // read and write */
        /// Flip RGBA to ARGB or GA to AG
        const SWAP_ALPHA          = 0x0100; // read and write */
        /// Byte-swap 16-bit samples
        const SWAP_ENDIAN         = 0x0200; // read and write */
        /// Change alpha from opacity to transparency
        const INVERT_ALPHA        = 0x0400; // read and write */
        const STRIP_FILLER        = 0x0800; // write only */
        const STRIP_FILLER_BEFORE = 0x0800; // write only
        const STRIP_FILLER_AFTER  = 0x1000; // write only */
        const GRAY_TO_RGB         = 0x2000; // read only */
        const EXPAND_16           = 0x4000; // read only */
        const SCALE_16            = 0x8000; // read only */
    }
}

#[derive(Debug)]
pub struct ParameterError {
    inner: ParameterErrorKind,
}

#[derive(Debug)]
pub(crate) enum ParameterErrorKind {
    /// A provided buffer must be have the exact size to hold the image data. Where the buffer can
    /// be allocated by the caller, they must ensure that it has a minimum size as hinted previously.
    /// Even though the size is calculated from image data, this does counts as a parameter error
    /// because they must react to a value produced by this library, which can have been subjected
    /// to limits.
    ImageBufferSize { expected: usize, actual: usize },
    /// A bit like return `None` from an iterator.
    /// We use it to differentiate between failing to seek to the next image in a sequence and the
    /// absence of a next image. This is an error of the caller because they should have checked
    /// the number of images by inspecting the header data returned when opening the image. This
    /// library will perform the checks necessary to ensure that data was accurate or error with a
    /// format error otherwise.
    PolledAfterEndOfImage,
}

impl From<ParameterErrorKind> for ParameterError {
    fn from(inner: ParameterErrorKind) -> Self {
        ParameterError { inner }
    }
}

impl fmt::Display for ParameterError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use ParameterErrorKind::*;
        match self.inner {
            ImageBufferSize { expected, actual } => {
                write!(fmt, "wrong data size, expected {} got {}", expected, actual)
            }
            PolledAfterEndOfImage => write!(fmt, "End of image has been reached"),
        }
    }
}

/// Mod to encapsulate the converters depending on the `deflate` crate.
///
/// Since this only contains trait impls, there is no need to make this public, they are simply
/// available when the mod is compiled as well.
#[cfg(feature = "png-encoding")]
mod deflate_convert {
    extern crate deflate;
    use super::Compression;

    impl From<deflate::Compression> for Compression {
        fn from(c: deflate::Compression) -> Self {
            match c {
                deflate::Compression::Default => Compression::Default,
                deflate::Compression::Fast => Compression::Fast,
                deflate::Compression::Best => Compression::Best,
            }
        }
    }

    impl From<Compression> for deflate::CompressionOptions {
        fn from(c: Compression) -> Self {
            match c {
                Compression::Default => deflate::CompressionOptions::default(),
                Compression::Fast => deflate::CompressionOptions::fast(),
                Compression::Best => deflate::CompressionOptions::high(),
                Compression::Huffman => deflate::CompressionOptions::huffman_only(),
                Compression::Rle => deflate::CompressionOptions::rle(),
            }
        }
    }
}
