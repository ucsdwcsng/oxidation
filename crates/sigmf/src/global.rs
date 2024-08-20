use super::{Integer, Number, SmfError};
use geojson::GeoJson;
use serde::{
    de::{self, Deserializer, Visitor},
    ser::{Serialize, SerializeStruct},
};
use serde_derive::Deserialize;
use std::fmt;

#[derive(Debug, Default, Deserialize)]
pub struct SmfGlobal {
    // Required fields
    #[serde(rename = "core:version")]
    pub version: String,
    #[serde(rename = "core:datatype")]
    pub datatype: SmfDataType,

    // Optional fields
    #[serde(rename = "core:sample_rate")]
    pub sample_rate: Option<Number>,
    #[serde(rename = "core:author")]
    pub author: Option<String>,
    #[serde(rename = "core:collection")]
    pub collection: Option<String>,
    #[serde(rename = "core:data_doi")]
    pub data_doi: Option<String>,
    #[serde(rename = "core:description")]
    pub description: Option<String>,
    #[serde(rename = "core:hw")]
    pub hw: Option<String>,
    #[serde(rename = "core:license")]
    pub license: Option<String>,
    #[serde(rename = "core:metadata_only")]
    pub metadata_only: Option<bool>, // Cannot be used in Non-Conforming Datasets
    #[serde(rename = "core:meta_doi")]
    pub meta_doi: Option<String>,
    #[serde(rename = "core:num_channels")]
    pub num_channels: Option<Integer>,
    #[serde(rename = "core:offset")]
    pub offset: Option<Integer>,
    #[serde(rename = "core:recorder")]
    pub recorder: Option<String>,
    #[serde(rename = "core:sha512")]
    pub sha512: Option<String>,
    #[serde(rename = "core:geolocation")]
    pub geolocation: Option<GeoJson>,
    #[serde(rename = "core:extensions")]
    pub extensions: Option<Vec<SmfExtension>>,

    // For Non-Conforming Datasets ONLY
    #[serde(rename = "core:dataset")]
    pub dataset: Option<String>,
    #[serde(rename = "core:trailing_bytes")]
    pub trailing_bytes: Option<Integer>,
}

impl Serialize for SmfGlobal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("SmfGlobal", 17)?;

        state.serialize_field("core:version", &self.version)?;
        state.serialize_field("core:datatype", &self.datatype)?;

        if let Some(sample_rate) = &self.sample_rate {
            state.serialize_field("core:sample_rate", sample_rate)?;
        }

        if let Some(author) = &self.author {
            state.serialize_field("core:author", author)?;
        }

        if let Some(collection) = &self.collection {
            state.serialize_field("core:collection", collection)?;
        }

        if let Some(data_doi) = &self.data_doi {
            state.serialize_field("core:data_doi", data_doi)?;
        }

        if let Some(description) = &self.description {
            state.serialize_field("core:description", description)?;
        }

        if let Some(hw) = &self.hw {
            state.serialize_field("core:hw", hw)?;
        }

        if let Some(license) = &self.license {
            state.serialize_field("core:license", license)?;
        }

        if let Some(metadata_only) = &self.metadata_only {
            state.serialize_field("core:metadata_only", metadata_only)?;
        }

        if let Some(meta_doi) = &self.meta_doi {
            state.serialize_field("core:meta_doi", meta_doi)?;
        }

        if let Some(num_channels) = &self.num_channels {
            state.serialize_field("core:num_channels", num_channels)?;
        }

        if let Some(offset) = &self.offset {
            state.serialize_field("core:offset", offset)?;
        }

        if let Some(recorder) = &self.recorder {
            state.serialize_field("core:recorder", recorder)?;
        }

        if let Some(sha512) = &self.sha512 {
            state.serialize_field("core:sha512", sha512)?;
        }

        if let Some(geolocation) = &self.geolocation {
            state.serialize_field("core:geolocation", geolocation)?;
        }

        if let Some(extensions) = &self.extensions {
            state.serialize_field("core:extensions", extensions)?;
        }

        if let Some(dataset) = &self.dataset {
            state.serialize_field("core:dataset", dataset)?;
        }

        if let Some(trailing_bytes) = &self.trailing_bytes {
            state.serialize_field("core:trailing_bytes", trailing_bytes)?;
        }

        state.end()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SmfDataType {
    pub domain: String,
    pub dtype: Option<String>,
    pub endian: Option<String>,
    pub byte: Option<String>,
}

pub enum SmfDataTypeDomain {
    Real,
    Complex,
}

pub enum SmfDataTypeDtype {
    F32,
    F64,
    I16,
    I32,
    U16,
    U32,
}

pub enum SmfDataTypeEndian {
    LE,
    BE,
}

pub enum SmfDataTypeByte {
    I8,
    U8,
}

impl SmfDataType {
    pub fn new(
        domain: SmfDataTypeDomain,
        dtype: Option<SmfDataTypeDtype>,
        endian: Option<SmfDataTypeEndian>,
        byte: Option<SmfDataTypeByte>,
    ) -> Self {
        let mut datatype = SmfDataType::default();

        datatype.set_domain(domain);

        if let Some(dtype) = dtype {
            datatype.set_dtype(dtype).unwrap();
        }

        if let Some(endian) = endian {
            datatype.set_endian(endian).unwrap();
        }

        if let Some(byte) = byte {
            datatype.set_byte(byte).unwrap();
        }

        datatype
    }

    pub fn set_domain(&mut self, domain: SmfDataTypeDomain) {
        self.domain = match domain {
            SmfDataTypeDomain::Real => "r".to_string(),
            SmfDataTypeDomain::Complex => "c".to_string(),
        };
    }

    pub fn set_dtype(&mut self, dtype: SmfDataTypeDtype) -> Result<(), SmfError> {
        if self.byte.is_some() {
            return Err(SmfError::InvalidDataType(
                "dtype and byte cannot appear together".into(),
            ));
        }
        self.dtype = match dtype {
            SmfDataTypeDtype::F32 => Some("f32".to_string()),
            SmfDataTypeDtype::F64 => Some("f64".to_string()),
            SmfDataTypeDtype::I16 => Some("i16".to_string()),
            SmfDataTypeDtype::I32 => Some("i32".to_string()),
            SmfDataTypeDtype::U16 => Some("u16".to_string()),
            SmfDataTypeDtype::U32 => Some("u32".to_string()),
        };

        Ok(())
    }

    pub fn set_endian(&mut self, endian: SmfDataTypeEndian) -> Result<(), SmfError> {
        if self.byte.is_some() {
            return Err(SmfError::InvalidDataType(
                "endian and byte cannot appear together".into(),
            ));
        }

        self.endian = match endian {
            SmfDataTypeEndian::LE => Some("le".to_string()),
            SmfDataTypeEndian::BE => Some("be".to_string()),
        };

        Ok(())
    }

    pub fn set_byte(&mut self, byte: SmfDataTypeByte) -> Result<(), SmfError> {
        if self.dtype.is_some() || self.endian.is_some() {
            return Err(SmfError::InvalidDataType(
                "byte must appear only if dtype and endian are absent".into(),
            ));
        }

        self.byte = match byte {
            SmfDataTypeByte::I8 => Some("i8".to_string()),
            SmfDataTypeByte::U8 => Some("u8".to_string()),
        };

        Ok(())
    }

    pub fn get_domain(&self) -> SmfDataTypeDomain {
        match self.domain.as_str() {
            "r" => SmfDataTypeDomain::Real,
            "c" => SmfDataTypeDomain::Complex,
            _ => unreachable!(),
        }
    }

    pub fn get_dtype(&self) -> Option<SmfDataTypeDtype> {
        self.dtype.as_ref().map(|dtype| match dtype.as_str() {
            "f32" => SmfDataTypeDtype::F32,
            "f64" => SmfDataTypeDtype::F64,
            "i16" => SmfDataTypeDtype::I16,
            "i32" => SmfDataTypeDtype::I32,
            "u16" => SmfDataTypeDtype::U16,
            "u32" => SmfDataTypeDtype::U32,
            _ => unreachable!(),
        })
    }

    pub fn get_endian(&self) -> Option<SmfDataTypeEndian> {
        self.endian.as_ref().map(|endian| match endian.as_str() {
            "le" => SmfDataTypeEndian::LE,
            "be" => SmfDataTypeEndian::BE,
            _ => unreachable!(),
        })
    }

    pub fn get_byte(&self) -> Option<SmfDataTypeByte> {
        self.byte.as_ref().map(|byte| match byte.as_str() {
            "i8" => SmfDataTypeByte::I8,
            "u8" => SmfDataTypeByte::U8,
            _ => unreachable!(),
        })
    }
}

impl serde::Serialize for SmfDataType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = self.domain.to_string();

        // Handle dtype and endian (they always appear together)
        if let Some(dtype) = &self.dtype {
            s.push_str(dtype);
            if let Some(endian) = &self.endian {
                s.push('_');
                s.push_str(endian);
            } else {
                return Err(serde::ser::Error::custom(
                    "dtype and endian must appear together",
                ));
            }
        } else {
            if self.endian.is_some() {
                return Err(serde::ser::Error::custom(
                    "dtype and endian must appear together",
                ));
            }
        }

        // Handle byte (only present if both dtype and endian are absent)
        if let Some(byte) = &self.byte {
            if self.dtype.is_some() || self.endian.is_some() {
                return Err(serde::ser::Error::custom(
                    "byte must be present only if dtype and endian are absent",
                ));
            }
            s.push_str(byte);
        }

        serializer.serialize_str(&s)
    }
}

impl<'de> serde::Deserialize<'de> for SmfDataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SmfDataTypeVisitor;

        impl<'de> Visitor<'de> for SmfDataTypeVisitor {
            type Value = SmfDataType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing SmfDataType")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let domain;
                let mut dtype = None;
                let mut endian = None;
                let mut byte = None;

                // Match the domain first
                if value.starts_with('r') {
                    domain = Some("r".to_string());
                } else if value.starts_with('c') {
                    domain = Some("c".to_string());
                } else {
                    return Err(E::custom(format!(
                        "Invalid format: Missing domain: {}",
                        value
                    )));
                }

                // Check for byte format
                if value.ends_with("i8") || value.ends_with("u8") {
                    byte = Some(
                        value
                            .chars()
                            .rev()
                            .take(2)
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect(),
                    );
                } else {
                    let dtype_endian = &value[1..].split('_').collect::<Vec<&str>>();
                    if dtype_endian.len() != 2 {
                        return Err(E::custom(format!("Invalid format: {}", value)));
                    }

                    let dtype_str = dtype_endian[0];
                    let endian_str = dtype_endian[1];

                    if !(dtype_str.ends_with("f32")
                        || dtype_str.ends_with("f64")
                        || dtype_str.ends_with("i16")
                        || dtype_str.ends_with("i32")
                        || dtype_str.ends_with("i64")
                        || dtype_str.ends_with("u16")
                        || dtype_str.ends_with("u32")
                        || dtype_str.ends_with("u64"))
                    {
                        return Err(E::custom(format!(
                            "Invalid format: Invalid dtype: {}",
                            dtype_str
                        )));
                    }
                    dtype = Some(dtype_str.to_string());

                    if !(endian_str.ends_with("le") || endian_str.ends_with("be")) {
                        return Err(E::custom(format!(
                            "Invalid format: Invalid endian: {}",
                            endian_str
                        )));
                    }
                    endian = Some(endian_str.to_string());
                }

                Ok(SmfDataType {
                    domain: domain.unwrap(),
                    dtype,
                    endian,
                    byte,
                })
            }
        }

        deserializer.deserialize_str(SmfDataTypeVisitor)
    }
}

#[derive(Debug, Clone, Deserialize, serde_derive::Serialize)]
pub struct SmfExtension {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_smf_data_type() {
        let valid_dtype1 = SmfDataType {
            domain: "c".to_string(),
            dtype: Some("f32".to_string()),
            endian: Some("le".to_string()),
            byte: None,
        };

        let valid_dtype2 = SmfDataType {
            domain: "r".to_string(),
            dtype: Some("u32".to_string()),
            endian: Some("be".to_string()),
            byte: None,
        };

        let valid_dtype3 = SmfDataType {
            domain: "c".to_string(),
            dtype: None,
            endian: None,
            byte: Some("u8".to_string()),
        };

        let invalid_dtype1 = SmfDataType {
            domain: "c".to_string(),
            dtype: Some("f32".to_string()),
            endian: None,
            byte: Some("u8".to_string()),
        };

        let invalid_dtype2 = SmfDataType {
            domain: "c".to_string(),
            dtype: None,
            endian: Some("le".to_string()),
            byte: None,
        };

        let valid_dtype1_str = r#""cf32_le""#;
        let valid_dtype2_str = r#""ru32_be""#;
        let valid_dtype3_str = r#""cu8""#;

        assert_eq!(
            serde_json::to_string(&valid_dtype1).unwrap(),
            valid_dtype1_str
        );
        assert_eq!(
            serde_json::to_string(&valid_dtype2).unwrap(),
            valid_dtype2_str
        );
        assert_eq!(
            serde_json::to_string(&valid_dtype3).unwrap(),
            valid_dtype3_str
        );
        assert!(serde_json::to_string(&invalid_dtype1).is_err());
        assert!(serde_json::to_string(&invalid_dtype2).is_err());
    }

    #[test]
    fn test_deserialize_smf_data_type() {
        let valid_dtype1 = SmfDataType {
            domain: "c".to_string(),
            dtype: Some("f32".to_string()),
            endian: Some("le".to_string()),
            byte: None,
        };

        let valid_dtype2 = SmfDataType {
            domain: "r".to_string(),
            dtype: Some("u32".to_string()),
            endian: Some("be".to_string()),
            byte: None,
        };

        let valid_dtype3 = SmfDataType {
            domain: "c".to_string(),
            dtype: None,
            endian: None,
            byte: Some("u8".to_string()),
        };

        let valid_dtype1_str = r#""cf32_le""#;
        let valid_dtype2_str = r#""ru32_be""#;
        let valid_dtype3_str = r#""cu8""#;

        assert_eq!(
            serde_json::from_str::<SmfDataType>(valid_dtype1_str).unwrap(),
            valid_dtype1
        );
        assert_eq!(
            serde_json::from_str::<SmfDataType>(valid_dtype2_str).unwrap(),
            valid_dtype2
        );
        assert_eq!(
            serde_json::from_str::<SmfDataType>(valid_dtype3_str).unwrap(),
            valid_dtype3
        );
        // assert!(serde_json::from_str::<SmfDataType>(invalid_dtype1).is_err());
        // assert!(serde_json::from_str::<SmfDataType>(invalid_dtype2).is_err());
        // assert!(serde_json::from_str::<SmfDataType>(invalid_dtype3).is_err());
    }
}
