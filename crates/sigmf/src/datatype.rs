use crate::SigmfError;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleDomain {
    Real,
    Complex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    F32,
    F64,
    I16,
    I32,
    U16,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    LE,
    BE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteType {
    I8,
    U8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datatype {
    wire_value: String,
}

impl Datatype {
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

    pub fn get_domain(&self) -> SampleDomain {
        match self.wire_value.as_bytes()[0] {
            b'r' => SampleDomain::Real,
            b'c' => SampleDomain::Complex,
            _ => unreachable!("Datatype can only be constructed from a validated wire value"),
        }
    }

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

    pub fn get_endian(&self) -> Option<Endianness> {
        match self.wire_value.rsplit_once('_').map(|(_, endian)| endian) {
            Some("le") => Some(Endianness::LE),
            Some("be") => Some(Endianness::BE),
            _ => None,
        }
    }

    pub fn get_byte(&self) -> Option<ByteType> {
        match self.wire_value.get(1..) {
            Some("i8") => Some(ByteType::I8),
            Some("u8") => Some(ByteType::U8),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.wire_value
    }

    pub fn is_complex(&self) -> bool {
        self.get_domain() == SampleDomain::Complex
    }

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

    pub fn bytes_per_channel_sample(&self) -> usize {
        self.component_bytes() * if self.is_complex() { 2 } else { 1 }
    }
}

impl FromStr for Datatype {
    type Err = SigmfError;

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
