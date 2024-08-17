use super::{Integer, Number};
use geojson::GeoJson;
use serde::de::{self, Deserializer, Visitor};
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

#[derive(Debug, Default, PartialEq)]
pub struct SmfDataType {
    pub domain: String,
    pub dtype: Option<String>,
    pub endian: Option<String>,
    pub byte: Option<String>,
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
                let mut domain = None;
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

#[derive(Debug, Deserialize)]
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

        let invalid_dtype1 = r#""cf32""#;
        let invalid_dtype2 = r#""cf32_le_u8""#;
        let invalid_dtype3 = r#""cle""#;

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
