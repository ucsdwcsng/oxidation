//! SigMF sample datatypes (`core:datatype`).
//!
//! A SigMF datatype string describes how each sample is stored in a
//! `.sigmf-data` file. It is composed of:
//!
//! * a domain prefix: `r` for real or `c` for complex (interleaved I/Q),
//! * a component type: `f32`, `f64`, `i32`, `i16`, `u32`, `u16`, `i8`, or `u8`,
//! * for multi-byte components, an endianness suffix: `_le` or `_be`.
//!
//! Examples include `cf32_le`, `ri16_be`, and `ru8`. All 28 core datatypes
//! defined by the SigMF specification are supported; other strings are
//! rejected. [`Datatype`] stores the validated string and exposes its parts
//! through [`SampleDomain`], [`ComponentType`], [`Endianness`], and
//! [`ByteType`].
//!
//! ```rust
//! use sigmf::datatype::{ComponentType, Datatype, Endianness, SampleDomain};
//! use std::str::FromStr;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let datatype = Datatype::from_str("cf32_le")?;
//! assert!(datatype.is_complex());
//! assert_eq!(datatype.get_domain(), SampleDomain::Complex);
//! assert_eq!(datatype.get_dtype(), Some(ComponentType::F32));
//! assert_eq!(datatype.get_endian(), Some(Endianness::LE));
//! assert_eq!(datatype.component_bytes(), 4);
//! assert_eq!(datatype.bytes_per_channel_sample(), 8);
//! assert_eq!(datatype.to_string(), "cf32_le");
//!
//! assert!(Datatype::from_str("cf32").is_err()); // missing endianness
//! # Ok(())
//! # }
//! ```

use crate::SigmfError;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::str::FromStr;

/// Whether samples are real-valued or complex-valued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleDomain {
    /// Real samples (`r` prefix); one component per sample.
    Real,
    /// Complex samples (`c` prefix); interleaved in-phase and quadrature
    /// components, two per sample.
    Complex,
}

/// Multi-byte component type of a datatype.
///
/// Single-byte components are described by [`ByteType`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// 32-bit IEEE 754 floating point (`f32`).
    F32,
    /// 64-bit IEEE 754 floating point (`f64`).
    F64,
    /// 16-bit signed integer (`i16`).
    I16,
    /// 32-bit signed integer (`i32`).
    I32,
    /// 16-bit unsigned integer (`u16`).
    U16,
    /// 32-bit unsigned integer (`u32`).
    U32,
}

/// Byte order of a multi-byte component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// Little-endian (`_le` suffix).
    LE,
    /// Big-endian (`_be` suffix).
    BE,
}

/// Single-byte component type of a datatype.
///
/// Single-byte datatypes such as `ri8` and `cu8` have no endianness suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteType {
    /// 8-bit signed integer (`i8`).
    I8,
    /// 8-bit unsigned integer (`u8`).
    U8,
}

/// A validated SigMF `core:datatype` value such as `cf32_le` or `ru8`.
///
/// Construct one by parsing a string with [`FromStr`] or from its parts with
/// [`Datatype::new`]. It serializes to and deserializes from its string form,
/// and deserialization rejects strings that are not core SigMF datatypes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datatype {
    wire_value: String,
}

impl Datatype {
    /// Builds a datatype from its domain and component description.
    ///
    /// Supply either `component` and `endianness` (for multi-byte types) or
    /// only `byte_type` (for single-byte types).
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidDataType`] if any other combination of
    /// `component`, `endianness`, and `byte_type` is given.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::datatype::{ByteType, ComponentType, Datatype, Endianness, SampleDomain};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let iq = Datatype::new(
    ///     SampleDomain::Complex,
    ///     Some(ComponentType::I16),
    ///     Some(Endianness::BE),
    ///     None,
    /// )?;
    /// assert_eq!(iq.as_str(), "ci16_be");
    ///
    /// let bytes = Datatype::new(SampleDomain::Real, None, None, Some(ByteType::U8))?;
    /// assert_eq!(bytes.as_str(), "ru8");
    ///
    /// assert!(Datatype::new(SampleDomain::Real, Some(ComponentType::F32), None, None).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        domain: SampleDomain,
        component: Option<ComponentType>,
        endianness: Option<Endianness>,
        byte_type: Option<ByteType>,
    ) -> Result<Self, SigmfError> {
        let domain = match domain {
            SampleDomain::Real => "r",
            SampleDomain::Complex => "c",
        };
        let wire_value = match (component, endianness, byte_type) {
            (Some(component), Some(endianness), None) => {
                let component = match component {
                    ComponentType::F32 => "f32",
                    ComponentType::F64 => "f64",
                    ComponentType::I16 => "i16",
                    ComponentType::I32 => "i32",
                    ComponentType::U16 => "u16",
                    ComponentType::U32 => "u32",
                };
                let endian = match endianness {
                    Endianness::LE => "le",
                    Endianness::BE => "be",
                };
                format!("{domain}{component}_{endian}")
            }
            (None, None, Some(byte_type)) => {
                let byte = match byte_type {
                    ByteType::I8 => "i8",
                    ByteType::U8 => "u8",
                };
                format!("{domain}{byte}")
            }
            _ => return Err(SigmfError::InvalidDataType(
                "datatype requires either an 8-bit byte type or a component type and endianness"
                    .to_string(),
            )),
        };
        Ok(Self { wire_value })
    }

    /// Returns whether samples are real or complex.
    pub fn get_domain(&self) -> SampleDomain {
        match self.wire_value.as_bytes()[0] {
            b'r' => SampleDomain::Real,
            b'c' => SampleDomain::Complex,
            _ => unreachable!("Datatype can only be constructed from a validated wire value"),
        }
    }

    /// Returns the multi-byte component type, or `None` for single-byte
    /// datatypes (see [`get_byte`](Self::get_byte)).
    pub fn get_dtype(&self) -> Option<ComponentType> {
        match self
            .wire_value
            .split('_')
            .next()
            .unwrap_or_default()
            .get(1..)
        {
            Some("f32") => Some(ComponentType::F32),
            Some("f64") => Some(ComponentType::F64),
            Some("i16") => Some(ComponentType::I16),
            Some("i32") => Some(ComponentType::I32),
            Some("u16") => Some(ComponentType::U16),
            Some("u32") => Some(ComponentType::U32),
            _ => None,
        }
    }

    /// Returns the byte order, or `None` for single-byte datatypes.
    pub fn get_endian(&self) -> Option<Endianness> {
        match self.wire_value.rsplit_once('_').map(|(_, endian)| endian) {
            Some("le") => Some(Endianness::LE),
            Some("be") => Some(Endianness::BE),
            _ => None,
        }
    }

    /// Returns the single-byte component type, or `None` for multi-byte
    /// datatypes (see [`get_dtype`](Self::get_dtype)).
    pub fn get_byte(&self) -> Option<ByteType> {
        match self.wire_value.get(1..) {
            Some("i8") => Some(ByteType::I8),
            Some("u8") => Some(ByteType::U8),
            _ => None,
        }
    }

    /// Returns the datatype string as it appears in SigMF metadata.
    pub fn as_str(&self) -> &str {
        &self.wire_value
    }

    /// Returns `true` if the datatype is complex (`c` prefix).
    pub fn is_complex(&self) -> bool {
        self.get_domain() == SampleDomain::Complex
    }

    /// Returns the size in bytes of one component (one real value, or one of
    /// the I/Q halves of a complex value).
    pub fn component_bytes(&self) -> usize {
        match self.get_byte() {
            Some(_) => 1,
            None => match self.get_dtype() {
                Some(ComponentType::F32 | ComponentType::I32 | ComponentType::U32) => 4,
                Some(ComponentType::F64) => 8,
                Some(ComponentType::I16 | ComponentType::U16) => 2,
                None => unreachable!("validated datatype has a component type"),
            },
        }
    }

    /// Returns the size in bytes of one sample on one channel.
    ///
    /// This is [`component_bytes`](Self::component_bytes) for real datatypes
    /// and twice that for complex datatypes. Multiply by `core:num_channels`
    /// to get the size of a frame.
    pub fn bytes_per_channel_sample(&self) -> usize {
        self.component_bytes() * if self.is_complex() { 2 } else { 1 }
    }
}

impl FromStr for Datatype {
    type Err = SigmfError;

    /// Parses a core SigMF datatype string such as `cf32_le` or `ri8`.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidDataType`] if the string is empty, has an
    /// unknown domain or component type, is missing or has an invalid
    /// endianness suffix, or adds a suffix to a single-byte type.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = value.as_bytes();
        let Some((&domain, components)) = bytes.split_first() else {
            return Err(SigmfError::InvalidDataType(
                "datatype must not be empty".to_string(),
            ));
        };
        if domain != b'r' && domain != b'c' {
            return Err(SigmfError::InvalidDataType(format!(
                "invalid datatype domain in {value:?}"
            )));
        }

        let body = std::str::from_utf8(components).map_err(|_| {
            SigmfError::InvalidDataType(format!("datatype is not ASCII: {value:?}"))
        })?;
        if matches!(body, "i8" | "u8") {
            return Ok(Self {
                wire_value: value.to_string(),
            });
        }
        let (component, endian) = body.split_once('_').ok_or_else(|| {
            SigmfError::InvalidDataType(format!("datatype {value:?} requires an endian suffix"))
        })?;
        if body.matches('_').count() != 1 {
            return Err(SigmfError::InvalidDataType(format!(
                "invalid datatype suffix in {value:?}"
            )));
        }
        if !matches!(component, "f32" | "f64" | "i16" | "i32" | "u16" | "u32") {
            return Err(SigmfError::InvalidDataType(format!(
                "unsupported core component type in {value:?}"
            )));
        }
        if !matches!(endian, "le" | "be") {
            return Err(SigmfError::InvalidDataType(format!(
                "invalid datatype byte order in {value:?}"
            )));
        }
        Ok(Self {
            wire_value: value.to_string(),
        })
    }
}

impl fmt::Display for Datatype {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for Datatype {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Datatype {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_28_core_datatypes_round_trip() {
        let components = [
            "i8", "u8", "f32_le", "f32_be", "f64_le", "f64_be", "i16_le", "i16_be", "i32_le",
            "i32_be", "u16_le", "u16_be", "u32_le", "u32_be",
        ];
        for domain in ["r", "c"] {
            for component in components {
                let value = format!("{domain}{component}");
                let datatype = Datatype::from_str(&value).unwrap();
                assert_eq!(datatype.to_string(), value);
                assert_eq!(
                    serde_json::to_string(&datatype).unwrap(),
                    format!("\"{value}\"")
                );
                assert_eq!(
                    serde_json::from_str::<Datatype>(&format!("\"{value}\"")).unwrap(),
                    datatype
                );
            }
        }
    }

    #[test]
    fn rejects_non_core_and_malformed_datatypes() {
        for value in [
            "",
            "xf32_le",
            "cf32_le_extra",
            "cf32",
            "ci8_le",
            "ci64_le",
            "rf32_xx",
            "cf32_le_extra",
            "cf32__le",
        ] {
            assert!(Datatype::from_str(value).is_err(), "accepted {value:?}");
        }
    }
}
