//! Encoding and decoding of raw SigMF sample data.
//!
//! A SigMF dataset is a sequence of *frames*. A frame holds one sample for
//! each of the `core:num_channels` channels, stored channel after channel;
//! complex samples store their real (I) component before their imaginary (Q)
//! component. The layout of each component is given by a
//! [`Datatype`].
//!
//! Decoded samples are represented as a flat list of [`SampleValue`]s in the
//! same order as on disk: frame 0 channel 0, frame 0 channel 1, …, frame 1
//! channel 0, and so on. A list of samples therefore always holds a multiple
//! of the channel count.
//!
//! * [`encode`] and [`decode`] convert whole in-memory blocks.
//! * [`SampleWriter`] and [`SampleReader`] stream frames to a [`Write`] or
//!   from a [`Read`].
//! * [`bytes_per_frame`] and [`frame_count`] compute dataset sizes.
//!
//! ```rust
//! use num_complex::Complex;
//! use sigmf::datatype::Datatype;
//! use sigmf::utils::data::{decode, encode, SampleValue};
//! use std::str::FromStr;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let datatype = Datatype::from_str("ci16_le")?;
//! // Two channels, two frames.
//! let samples = vec![
//!     SampleValue::ComplexI16(Complex::new(1, -1)),
//!     SampleValue::ComplexI16(Complex::new(2, -2)),
//!     SampleValue::ComplexI16(Complex::new(3, -3)),
//!     SampleValue::ComplexI16(Complex::new(4, -4)),
//! ];
//! let bytes = encode(&samples, &datatype, 2)?;
//! assert_eq!(bytes.len(), 4 * 2 * 2);
//! assert_eq!(&bytes[..4], &[1, 0, 0xff, 0xff]);
//! assert_eq!(decode(&bytes, &datatype, 2)?, samples);
//! # Ok(())
//! # }
//! ```

use crate::SigmfError;
use crate::datatype::{ByteType, ComponentType, Datatype, Endianness, SampleDomain};
use num_complex::Complex;
use std::io::{Read, Write};

/// One sample on one channel, typed by its datatype.
///
/// The variant must match the domain and component type of the
/// [`Datatype`] it is encoded with; endianness is not part of the value. For
/// example, both `cf32_le` and `cf32_be` use [`SampleValue::ComplexF32`].
#[derive(Debug, PartialEq)]
pub enum SampleValue {
    /// A `cu8` sample.
    ComplexUnsignedBytes(Complex<u8>),
    /// A `ci8` sample.
    ComplexSignedBytes(Complex<i8>),
    /// A `cf32_le` or `cf32_be` sample.
    ComplexF32(Complex<f32>),
    /// A `cf64_le` or `cf64_be` sample.
    ComplexF64(Complex<f64>),
    /// A `ci16_le` or `ci16_be` sample.
    ComplexI16(Complex<i16>),
    /// A `ci32_le` or `ci32_be` sample.
    ComplexI32(Complex<i32>),
    /// A `cu16_le` or `cu16_be` sample.
    ComplexU16(Complex<u16>),
    /// A `cu32_le` or `cu32_be` sample.
    ComplexU32(Complex<u32>),
    /// An `ru8` sample.
    RealUnsignedBytes(u8),
    /// An `ri8` sample.
    RealSignedBytes(i8),
    /// An `rf32_le` or `rf32_be` sample.
    RealF32(f32),
    /// An `rf64_le` or `rf64_be` sample.
    RealF64(f64),
    /// An `ri16_le` or `ri16_be` sample.
    RealI16(i16),
    /// An `ri32_le` or `ri32_be` sample.
    RealI32(i32),
    /// An `ru16_le` or `ru16_be` sample.
    RealU16(u16),
    /// An `ru32_le` or `ru32_be` sample.
    RealU32(u32),
}

/// Decodes a block of complete frames into samples.
///
/// `data` must contain a whole number of frames for `datatype` and
/// `channel_count` channels. The result holds one [`SampleValue`] per channel
/// per frame, in on-disk order. See the [module documentation](self) for an
/// example.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero, the
/// frame size overflows, or `data` ends with an incomplete frame.
#[allow(unknown_lints)]
#[allow(clippy::manual_is_multiple_of)]
pub fn decode(
    data: &[u8],
    datatype: &Datatype,
    channel_count: usize,
) -> Result<Vec<SampleValue>, SigmfError> {
    if channel_count == 0 {
        return Err(SigmfError::InvalidMetadata(
            "core:num_channels must be greater than zero".to_string(),
        ));
    }

    let components_per_frame = channel_count
        .checked_mul(if datatype.is_complex() { 2 } else { 1 })
        .ok_or_else(|| SigmfError::InvalidMetadata("frame size overflow".to_string()))?;
    let bytes_per_frame = components_per_frame
        .checked_mul(datatype.component_bytes())
        .ok_or_else(|| SigmfError::InvalidMetadata("frame size overflow".to_string()))?;
    if data.len() % bytes_per_frame != 0 {
        return Err(SigmfError::InvalidMetadata(format!(
            "dataset ends with an incomplete frame at byte {}; frame size is {bytes_per_frame}",
            data.len() - data.len() % bytes_per_frame
        )));
    }

    let components = decode_components(data, datatype)?;
    if datatype.get_domain() == SampleDomain::Real {
        Ok(components)
    } else {
        pair_complex_components(components, datatype)
    }
}

/// Encodes complete frames of samples into raw bytes.
///
/// `samples` is in on-disk order (see the [module documentation](self)) and
/// its length must be a multiple of `channel_count`. Every sample must use
/// the [`SampleValue`] variant matching `datatype`.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero or the
/// samples do not form complete frames, [`SigmfError::InvalidDataType`] if a
/// sample does not match `datatype`, or [`SigmfError::ResourceLimit`] if the
/// encoded size overflows.
#[allow(unknown_lints)]
#[allow(clippy::manual_is_multiple_of)]
pub fn encode(
    samples: &[SampleValue],
    datatype: &Datatype,
    channel_count: usize,
) -> Result<Vec<u8>, SigmfError> {
    if channel_count == 0 {
        return Err(SigmfError::InvalidMetadata(
            "core:num_channels must be greater than zero".to_string(),
        ));
    }
    if samples.len() % channel_count != 0 {
        return Err(SigmfError::InvalidMetadata(format!(
            "{} channel samples do not form complete {channel_count}-channel frames",
            samples.len()
        )));
    }
    let capacity = samples
        .len()
        .checked_mul(datatype.bytes_per_channel_sample())
        .ok_or_else(|| SigmfError::ResourceLimit("encoded sample block is too large".into()))?;

    macro_rules! encode_scalars {
        ($ty:ty, $real_variant:ident, $complex_variant:ident, $to_bytes:ident) => {{
            let mut encoded = Vec::with_capacity(capacity);
            for sample in samples {
                match (datatype.get_domain(), sample) {
                    (SampleDomain::Real, SampleValue::$real_variant(value)) => {
                        encoded.extend_from_slice(&value.$to_bytes());
                    }
                    (SampleDomain::Complex, SampleValue::$complex_variant(value)) => {
                        let real: $ty = value.re;
                        let imaginary: $ty = value.im;
                        encoded.extend_from_slice(&real.$to_bytes());
                        encoded.extend_from_slice(&imaginary.$to_bytes());
                    }
                    _ => {
                        return Err(SigmfError::InvalidDataType(format!(
                            "sample value does not match datatype {}",
                            datatype
                        )));
                    }
                }
            }
            encoded
        }};
    }

    let encoded = match (
        datatype.get_byte(),
        datatype.get_dtype(),
        datatype.get_endian(),
    ) {
        (Some(ByteType::U8), None, None) => {
            encode_scalars!(u8, RealUnsignedBytes, ComplexUnsignedBytes, to_ne_bytes)
        }
        (Some(ByteType::I8), None, None) => {
            encode_scalars!(i8, RealSignedBytes, ComplexSignedBytes, to_ne_bytes)
        }
        (None, Some(ComponentType::F32), Some(Endianness::LE)) => {
            encode_scalars!(f32, RealF32, ComplexF32, to_le_bytes)
        }
        (None, Some(ComponentType::F32), Some(Endianness::BE)) => {
            encode_scalars!(f32, RealF32, ComplexF32, to_be_bytes)
        }
        (None, Some(ComponentType::F64), Some(Endianness::LE)) => {
            encode_scalars!(f64, RealF64, ComplexF64, to_le_bytes)
        }
        (None, Some(ComponentType::F64), Some(Endianness::BE)) => {
            encode_scalars!(f64, RealF64, ComplexF64, to_be_bytes)
        }
        (None, Some(ComponentType::I16), Some(Endianness::LE)) => {
            encode_scalars!(i16, RealI16, ComplexI16, to_le_bytes)
        }
        (None, Some(ComponentType::I16), Some(Endianness::BE)) => {
            encode_scalars!(i16, RealI16, ComplexI16, to_be_bytes)
        }
        (None, Some(ComponentType::I32), Some(Endianness::LE)) => {
            encode_scalars!(i32, RealI32, ComplexI32, to_le_bytes)
        }
        (None, Some(ComponentType::I32), Some(Endianness::BE)) => {
            encode_scalars!(i32, RealI32, ComplexI32, to_be_bytes)
        }
        (None, Some(ComponentType::U16), Some(Endianness::LE)) => {
            encode_scalars!(u16, RealU16, ComplexU16, to_le_bytes)
        }
        (None, Some(ComponentType::U16), Some(Endianness::BE)) => {
            encode_scalars!(u16, RealU16, ComplexU16, to_be_bytes)
        }
        (None, Some(ComponentType::U32), Some(Endianness::LE)) => {
            encode_scalars!(u32, RealU32, ComplexU32, to_le_bytes)
        }
        (None, Some(ComponentType::U32), Some(Endianness::BE)) => {
            encode_scalars!(u32, RealU32, ComplexU32, to_be_bytes)
        }
        _ => {
            return Err(SigmfError::InvalidDataType(
                "unsupported or invalid datatype representation".to_string(),
            ));
        }
    };
    Ok(encoded)
}

/// Streams frames of samples into a [`Write`] destination.
///
/// Each call to [`write_frames`](Self::write_frames) encodes its samples with
/// [`encode`] and writes them immediately; wrap the destination in a
/// [`std::io::BufWriter`] when writing many small blocks to a file.
///
/// # Examples
///
/// ```rust
/// use sigmf::datatype::Datatype;
/// use sigmf::utils::data::{SampleReader, SampleValue, SampleWriter};
/// use std::io::Cursor;
/// use std::str::FromStr;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let datatype = Datatype::from_str("rf32_be")?;
/// let mut writer = SampleWriter::new(Vec::new(), datatype.clone(), 1)?;
/// writer.write_frames(&[SampleValue::RealF32(0.5), SampleValue::RealF32(-1.0)])?;
/// writer.write_frames(&[SampleValue::RealF32(2.0)])?;
/// let bytes = writer.finish()?;
/// assert_eq!(bytes.len(), 12);
///
/// let mut reader = SampleReader::new(Cursor::new(bytes), datatype, 1)?;
/// assert_eq!(
///     reader.read_frames(2)?,
///     Some(vec![SampleValue::RealF32(0.5), SampleValue::RealF32(-1.0)])
/// );
/// assert_eq!(reader.read_frames(2)?, Some(vec![SampleValue::RealF32(2.0)]));
/// assert_eq!(reader.read_frames(2)?, None);
/// # Ok(())
/// # }
/// ```
pub struct SampleWriter<W> {
    writer: W,
    datatype: Datatype,
    channel_count: usize,
}

/// Streams frames of samples out of a [`Read`] source.
///
/// See [`SampleWriter`] for an example.
pub struct SampleReader<R> {
    reader: R,
    datatype: Datatype,
    channel_count: usize,
    frame_size: usize,
    bytes_read: u64,
}

impl<R: Read> SampleReader<R> {
    /// Creates a reader that decodes `channel_count`-channel frames of
    /// `datatype` from `reader`.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero or
    /// the frame size overflows.
    pub fn new(reader: R, datatype: Datatype, channel_count: usize) -> Result<Self, SigmfError> {
        let frame_size = bytes_per_frame(&datatype, channel_count)?;
        Ok(Self {
            reader,
            datatype,
            channel_count,
            frame_size,
            bytes_read: 0,
        })
    }

    #[allow(unknown_lints)]
    #[allow(clippy::manual_is_multiple_of)]
    /// Reads and decodes up to `maximum_frames` frames.
    ///
    /// Short and interrupted reads are retried until `maximum_frames` frames
    /// are available or the source reaches end of input. Returns
    /// `Ok(Some(samples))` with between one and `maximum_frames` frames (one
    /// [`SampleValue`] per channel per frame), or `Ok(None)` once the source is
    /// exhausted. A buffer for `maximum_frames` frames is allocated on each
    /// call.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidRange`] if `maximum_frames` is zero,
    /// [`SigmfError::ResourceLimit`] if the block size overflows,
    /// [`SigmfError::InvalidMetadata`] if the source ends with an incomplete
    /// frame, or [`SigmfError::Io`] if reading fails.
    pub fn read_frames(
        &mut self,
        maximum_frames: usize,
    ) -> Result<Option<Vec<SampleValue>>, SigmfError> {
        if maximum_frames == 0 {
            return Err(SigmfError::InvalidRange(
                "sample reader block size must be greater than zero".into(),
            ));
        }
        let maximum_bytes = self
            .frame_size
            .checked_mul(maximum_frames)
            .ok_or_else(|| SigmfError::ResourceLimit("sample block is too large".into()))?;
        let mut bytes = vec![0; maximum_bytes];
        let mut filled = 0usize;
        while filled < maximum_bytes {
            match self.reader.read(&mut bytes[filled..]) {
                Ok(0) => break,
                Ok(count) => {
                    filled += count;
                    self.bytes_read = self
                        .bytes_read
                        .checked_add(count as u64)
                        .ok_or_else(|| SigmfError::InvalidRange("byte offset overflow".into()))?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(SigmfError::Io(error)),
            }
        }
        if filled == 0 {
            return Ok(None);
        }
        if filled % self.frame_size != 0 {
            let incomplete_start = self.bytes_read - (filled % self.frame_size) as u64;
            return Err(SigmfError::InvalidMetadata(format!(
                "dataset ends with an incomplete frame at byte {incomplete_start}; frame size is {}",
                self.frame_size
            )));
        }
        bytes.truncate(filled);
        decode(&bytes, &self.datatype, self.channel_count).map(Some)
    }
}

impl<W: Write> SampleWriter<W> {
    /// Creates a writer that encodes `channel_count`-channel frames of
    /// `datatype` into `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero.
    pub fn new(writer: W, datatype: Datatype, channel_count: usize) -> Result<Self, SigmfError> {
        if channel_count == 0 {
            return Err(SigmfError::InvalidMetadata(
                "core:num_channels must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            writer,
            datatype,
            channel_count,
        })
    }

    /// Encodes and writes one block of complete frames.
    ///
    /// # Errors
    ///
    /// Returns any error from [`encode`] (in which case nothing is written),
    /// or [`SigmfError::Io`] if writing fails.
    pub fn write_frames(&mut self, samples: &[SampleValue]) -> Result<(), SigmfError> {
        let encoded = encode(samples, &self.datatype, self.channel_count)?;
        self.writer.write_all(&encoded)?;
        Ok(())
    }

    /// Flushes the destination and returns it.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::Io`] if flushing fails.
    pub fn finish(mut self) -> Result<W, SigmfError> {
        self.writer.flush()?;
        Ok(self.writer)
    }
}

fn decode_components(data: &[u8], datatype: &Datatype) -> Result<Vec<SampleValue>, SigmfError> {
    macro_rules! scalar_values {
        ($ty:ty, $width:expr, $decode:expr, $real:ident, $complex:ident) => {{
            if data.len() % $width != 0 {
                return Err(incomplete_component(data.len(), $width));
            }
            data.chunks_exact($width)
                .map(|chunk| {
                    let value: $ty = ($decode)(chunk);
                    if datatype.get_domain() == SampleDomain::Real {
                        SampleValue::$real(value)
                    } else {
                        SampleValue::$complex(Complex::new(value, <$ty>::default()))
                    }
                })
                .collect()
        }};
    }

    let values = match (
        datatype.get_byte(),
        datatype.get_dtype(),
        datatype.get_endian(),
    ) {
        (Some(ByteType::U8), None, None) => data
            .iter()
            .map(|&value| {
                if datatype.get_domain() == SampleDomain::Real {
                    SampleValue::RealUnsignedBytes(value)
                } else {
                    SampleValue::ComplexUnsignedBytes(Complex::new(value, 0))
                }
            })
            .collect(),
        (Some(ByteType::I8), None, None) => data
            .iter()
            .map(|&value| {
                let value = i8::from_ne_bytes([value]);
                if datatype.get_domain() == SampleDomain::Real {
                    SampleValue::RealSignedBytes(value)
                } else {
                    SampleValue::ComplexSignedBytes(Complex::new(value, 0))
                }
            })
            .collect(),
        (None, Some(ComponentType::F32), Some(Endianness::LE)) => {
            scalar_values!(
                f32,
                4,
                |b: &[u8]| f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
                RealF32,
                ComplexF32
            )
        }
        (None, Some(ComponentType::F32), Some(Endianness::BE)) => {
            scalar_values!(
                f32,
                4,
                |b: &[u8]| f32::from_be_bytes([b[0], b[1], b[2], b[3]]),
                RealF32,
                ComplexF32
            )
        }
        (None, Some(ComponentType::F64), Some(Endianness::LE)) => {
            scalar_values!(
                f64,
                8,
                |b: &[u8]| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
                RealF64,
                ComplexF64
            )
        }
        (None, Some(ComponentType::F64), Some(Endianness::BE)) => {
            scalar_values!(
                f64,
                8,
                |b: &[u8]| f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
                RealF64,
                ComplexF64
            )
        }
        (None, Some(ComponentType::I16), Some(Endianness::LE)) => {
            scalar_values!(
                i16,
                2,
                |b: &[u8]| i16::from_le_bytes([b[0], b[1]]),
                RealI16,
                ComplexI16
            )
        }
        (None, Some(ComponentType::I16), Some(Endianness::BE)) => {
            scalar_values!(
                i16,
                2,
                |b: &[u8]| i16::from_be_bytes([b[0], b[1]]),
                RealI16,
                ComplexI16
            )
        }
        (None, Some(ComponentType::I32), Some(Endianness::LE)) => {
            scalar_values!(
                i32,
                4,
                |b: &[u8]| i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
                RealI32,
                ComplexI32
            )
        }
        (None, Some(ComponentType::I32), Some(Endianness::BE)) => {
            scalar_values!(
                i32,
                4,
                |b: &[u8]| i32::from_be_bytes([b[0], b[1], b[2], b[3]]),
                RealI32,
                ComplexI32
            )
        }
        (None, Some(ComponentType::U16), Some(Endianness::LE)) => {
            scalar_values!(
                u16,
                2,
                |b: &[u8]| u16::from_le_bytes([b[0], b[1]]),
                RealU16,
                ComplexU16
            )
        }
        (None, Some(ComponentType::U16), Some(Endianness::BE)) => {
            scalar_values!(
                u16,
                2,
                |b: &[u8]| u16::from_be_bytes([b[0], b[1]]),
                RealU16,
                ComplexU16
            )
        }
        (None, Some(ComponentType::U32), Some(Endianness::LE)) => {
            scalar_values!(
                u32,
                4,
                |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
                RealU32,
                ComplexU32
            )
        }
        (None, Some(ComponentType::U32), Some(Endianness::BE)) => {
            scalar_values!(
                u32,
                4,
                |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
                RealU32,
                ComplexU32
            )
        }
        _ => {
            return Err(SigmfError::InvalidDataType(
                "unsupported or invalid datatype representation".to_string(),
            ));
        }
    };
    Ok(values)
}

#[allow(unknown_lints)]
#[allow(clippy::manual_is_multiple_of)]
fn pair_complex_components(
    components: Vec<SampleValue>,
    datatype: &Datatype,
) -> Result<Vec<SampleValue>, SigmfError> {
    if components.len() % 2 != 0 {
        return Err(incomplete_component(
            components.len() * datatype.component_bytes(),
            datatype.component_bytes() * 2,
        ));
    }
    let mut values = Vec::with_capacity(components.len() / 2);
    let mut components = components.into_iter();
    while let (Some(real), Some(imaginary)) = (components.next(), components.next()) {
        let value = match (real, imaginary) {
            (
                SampleValue::ComplexUnsignedBytes(real),
                SampleValue::ComplexUnsignedBytes(imaginary),
            ) => SampleValue::ComplexUnsignedBytes(Complex::new(real.re, imaginary.re)),
            (SampleValue::ComplexSignedBytes(real), SampleValue::ComplexSignedBytes(imaginary)) => {
                SampleValue::ComplexSignedBytes(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexF32(real), SampleValue::ComplexF32(imaginary)) => {
                SampleValue::ComplexF32(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexF64(real), SampleValue::ComplexF64(imaginary)) => {
                SampleValue::ComplexF64(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexI16(real), SampleValue::ComplexI16(imaginary)) => {
                SampleValue::ComplexI16(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexI32(real), SampleValue::ComplexI32(imaginary)) => {
                SampleValue::ComplexI32(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexU16(real), SampleValue::ComplexU16(imaginary)) => {
                SampleValue::ComplexU16(Complex::new(real.re, imaginary.re))
            }
            (SampleValue::ComplexU32(real), SampleValue::ComplexU32(imaginary)) => {
                SampleValue::ComplexU32(Complex::new(real.re, imaginary.re))
            }
            _ => {
                return Err(SigmfError::InvalidDataType(
                    "inconsistent complex component types".to_string(),
                ));
            }
        };
        values.push(value);
    }
    Ok(values)
}

fn incomplete_component(actual: usize, expected: usize) -> SigmfError {
    SigmfError::InvalidMetadata(format!(
        "dataset ends with an incomplete sample at byte {actual}; component/frame size is {expected}"
    ))
}

/// Returns the size in bytes of one frame of `channel_count` channels.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero or the
/// size overflows.
///
/// # Examples
///
/// ```rust
/// use sigmf::datatype::Datatype;
/// use sigmf::utils::data::{bytes_per_frame, frame_count};
/// use std::str::FromStr;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let datatype = Datatype::from_str("cf32_le")?;
/// assert_eq!(bytes_per_frame(&datatype, 4)?, 32);
/// assert_eq!(frame_count(320, &datatype, 4)?, 10);
/// assert!(frame_count(321, &datatype, 4).is_err());
/// # Ok(())
/// # }
/// ```
pub fn bytes_per_frame(datatype: &Datatype, channel_count: usize) -> Result<usize, SigmfError> {
    if channel_count == 0 {
        return Err(SigmfError::InvalidMetadata(
            "core:num_channels must be greater than zero".to_string(),
        ));
    }
    datatype
        .bytes_per_channel_sample()
        .checked_mul(channel_count)
        .ok_or_else(|| SigmfError::InvalidMetadata("frame size overflow".to_string()))
}

/// Returns the number of frames in `dataset_bytes` bytes of sample data.
///
/// `dataset_bytes` must exclude any capture header and trailing bytes.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if `channel_count` is zero, the
/// frame size overflows, or `dataset_bytes` is not a whole number of frames.
#[allow(unknown_lints)]
#[allow(clippy::manual_is_multiple_of)]
pub fn frame_count(
    dataset_bytes: u64,
    datatype: &Datatype,
    channel_count: usize,
) -> Result<u64, SigmfError> {
    let frame_size = bytes_per_frame(datatype, channel_count)? as u64;
    if dataset_bytes % frame_size != 0 {
        return Err(SigmfError::InvalidMetadata(format!(
            "dataset size {dataset_bytes} is not aligned to {frame_size}-byte frames"
        )));
    }
    Ok(dataset_bytes / frame_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::str::FromStr;

    #[derive(Default)]
    struct ShortWriter(Vec<u8>);

    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let Some(byte) = bytes.first() else {
                return Ok(0);
            };
            self.0.push(*byte);
            Ok(1)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct OneByteReader(io::Cursor<Vec<u8>>);

    impl Read for OneByteReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() {
                return Ok(0);
            }
            self.0.read(&mut buffer[..1])
        }
    }

    #[test]
    fn sample_reader_handles_one_byte_reads_and_rejects_truncated_frames() {
        let datatype = Datatype::from_str("ru8").unwrap();
        let mut reader =
            SampleReader::new(OneByteReader(io::Cursor::new(vec![1, 2, 3])), datatype, 1).unwrap();
        assert_eq!(
            reader.read_frames(2).unwrap().unwrap(),
            [
                SampleValue::RealUnsignedBytes(1),
                SampleValue::RealUnsignedBytes(2)
            ]
        );
        assert_eq!(
            reader.read_frames(2).unwrap().unwrap(),
            [SampleValue::RealUnsignedBytes(3)]
        );
        assert!(reader.read_frames(2).unwrap().is_none());

        let complex = Datatype::from_str("ci16_be").unwrap();
        let mut reader =
            SampleReader::new(OneByteReader(io::Cursor::new(vec![1, 2, 3])), complex, 1).unwrap();
        assert!(matches!(
            reader.read_frames(1),
            Err(SigmfError::InvalidMetadata(message)) if message.contains("incomplete frame")
        ));
    }

    #[test]
    fn real_byte_samples_preserve_every_input_byte() {
        let datatype = Datatype::from_str("ru8").unwrap();
        assert_eq!(
            decode(&[1, 2, 255], &datatype, 1).unwrap(),
            vec![
                SampleValue::RealUnsignedBytes(1),
                SampleValue::RealUnsignedBytes(2),
                SampleValue::RealUnsignedBytes(255)
            ]
        );
    }

    #[test]
    fn complex_values_follow_declared_big_endian_and_iq_order() {
        let datatype = Datatype::from_str("ci16_be").unwrap();
        assert_eq!(
            decode(&[0x01, 0x02, 0xfe, 0xdc], &datatype, 1).unwrap(),
            vec![SampleValue::ComplexI16(Complex::new(
                0x0102,
                0xfedc_u16 as i16
            ))]
        );
    }

    #[test]
    fn complex_and_multichannel_incomplete_frames_are_errors() {
        let complex = Datatype::from_str("ci8").unwrap();
        assert!(decode(&[1], &complex, 1).is_err());

        let real = Datatype::from_str("ri16_le").unwrap();
        assert!(decode(&[1, 2, 3, 4, 5, 6], &real, 2).is_err());
    }

    #[test]
    fn frame_count_uses_channels_and_samples_not_components() {
        let datatype = Datatype::from_str("ci16_le").unwrap();
        assert_eq!(frame_count(16, &datatype, 2).unwrap(), 2);
        assert!(frame_count(15, &datatype, 2).is_err());
    }

    #[test]
    fn codecs_round_trip_every_core_datatype() {
        let components = [
            "i8", "u8", "f32_le", "f32_be", "f64_le", "f64_be", "i16_le", "i16_be", "i32_le",
            "i32_be", "u16_le", "u16_be", "u32_le", "u32_be",
        ];
        for domain in ["r", "c"] {
            for component in components {
                let datatype = Datatype::from_str(&format!("{domain}{component}")).unwrap();
                let channels = 2;
                let frame_bytes = bytes_per_frame(&datatype, channels).unwrap();
                let source: Vec<_> = (0..frame_bytes * 3)
                    .map(|index| (index.wrapping_mul(73).wrapping_add(19)) as u8)
                    .collect();
                let decoded = decode(&source, &datatype, channels).unwrap();
                assert_eq!(
                    encode(&decoded, &datatype, channels).unwrap(),
                    source,
                    "codec round trip failed for {datatype}"
                );
            }
        }
    }

    #[test]
    fn encoder_rejects_mismatched_values_and_incomplete_channel_frames() {
        let real = Datatype::from_str("ri16_le").unwrap();
        assert!(encode(&[SampleValue::RealI32(1)], &real, 1).is_err());
        assert!(encode(&[SampleValue::RealI16(1)], &real, 2).is_err());
    }

    #[test]
    fn sample_writer_handles_short_writes_and_flushes_on_finish() {
        let datatype = Datatype::from_str("ri16_be").unwrap();
        let mut writer = SampleWriter::new(ShortWriter::default(), datatype, 1).unwrap();
        writer
            .write_frames(&[SampleValue::RealI16(0x1234), SampleValue::RealI16(-2)])
            .unwrap();
        assert_eq!(writer.finish().unwrap().0, [0x12, 0x34, 0xff, 0xfe]);
    }
}
