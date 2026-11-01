//! Capture segments: entries of the SigMF `captures` array.
//!
//! Each [`Capture`] describes a contiguous run of samples in the dataset,
//! beginning at `core:sample_start` and continuing until the next capture's
//! start (or the end of the dataset). Captures carry per-segment parameters
//! such as center frequency (`core:frequency`), wall-clock time
//! (`core:datetime`), and the number of non-sample header bytes that precede
//! the segment in the data file (`core:header_bytes`).
//!
//! Fields not defined by the SigMF core namespace (for example extension keys
//! like `"vendor:gain"`) are preserved verbatim and are reachable through
//! [`Capture::extra_fields`] and [`Capture::insert_extra`].

use super::{Integer, Number};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A single capture segment from the SigMF `captures` array.
///
/// Serializes to and from a JSON object keyed by SigMF field names. Optional
/// fields set to `None` are omitted from the output; unknown keys are
/// preserved in a flattened map of extension fields.
///
/// # Examples
///
/// ```rust
/// use sigmf::capture::Capture;
///
/// let mut capture = Capture::new(0);
/// capture.set_frequency(915e6);
/// capture.set_datetime("2026-01-01T00:00:00Z".to_string());
///
/// let json = serde_json::to_value(&capture).unwrap();
/// assert_eq!(json["core:sample_start"], 0);
/// assert_eq!(json["core:frequency"], 915e6);
/// assert!(json.get("core:header_bytes").is_none());
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Capture {
    // Required fields
    /// Index of the first sample of this segment within the dataset
    /// (`core:sample_start`); required.
    #[serde(rename = "core:sample_start")]
    pub sample_start: Integer,

    // Optional fields
    /// ISO 8601 timestamp of the segment's first sample (`core:datetime`).
    #[serde(rename = "core:datetime", skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    /// Center frequency of the signal in Hz (`core:frequency`).
    #[serde(rename = "core:frequency", skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Number>,
    /// Sample index of this segment's first sample in a larger, global
    /// sample stream such as an SDR's internal counter (`core:global_index`).
    #[serde(rename = "core:global_index", skip_serializing_if = "Option::is_none")]
    pub global_index: Option<Integer>,
    /// Number of bytes of non-sample header data preceding this segment in
    /// the dataset file (`core:header_bytes`).
    #[serde(rename = "core:header_bytes", skip_serializing_if = "Option::is_none")]
    pub header_bytes: Option<Integer>,
    /// GeoJSON `Point` giving the recording location for this segment
    /// (`core:geolocation`).
    #[serde(rename = "core:geolocation", skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<GeoJson>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Capture {
    /// Creates a capture starting at `sample_start` with every optional field
    /// unset and no extension fields.
    pub fn new(sample_start: Integer) -> Self {
        Capture {
            sample_start,
            datetime: None,
            frequency: None,
            global_index: None,
            header_bytes: None,
            geolocation: None,
            extra_fields: Map::new(),
        }
    }

    /// Returns the non-core (extension) fields preserved on this capture,
    /// keyed by their full JSON name (for example `"vendor:gain"`).
    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn extension_fields_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extra_fields
    }

    /// Inserts or replaces an extension field on this capture.
    ///
    /// Returns the previous value stored under `key`, if any. The key is
    /// stored verbatim; no namespace or extension-declaration check is
    /// performed here.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`] if `key` names a core
    /// capture field modeled by [`Capture`] (`core:sample_start`,
    /// `core:datetime`, `core:frequency`, `core:global_index`,
    /// `core:header_bytes`, or `core:geolocation`); set those through the
    /// typed fields or setters instead.
    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if matches!(
            key.as_str(),
            "core:sample_start"
                | "core:datetime"
                | "core:frequency"
                | "core:global_index"
                | "core:header_bytes"
                | "core:geolocation"
        ) {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    // Setters
    /// Sets `core:datetime`, the ISO 8601 timestamp of the first sample.
    ///
    /// The string is stored as given and is not validated.
    pub fn set_datetime(&mut self, datetime: String) {
        self.datetime = Some(datetime);
    }

    /// Sets `core:frequency`, the center frequency in Hz.
    pub fn set_frequency(&mut self, frequency: Number) {
        self.frequency = Some(frequency);
    }

    /// Sets `core:global_index`, the segment's index in a global sample
    /// stream.
    pub fn set_global_index(&mut self, global_index: Integer) {
        self.global_index = Some(global_index);
    }

    /// Sets `core:header_bytes`, the number of header bytes preceding this
    /// segment in the dataset file.
    pub fn set_header_bytes(&mut self, header_bytes: Integer) {
        self.header_bytes = Some(header_bytes);
    }

    /// Sets `core:geolocation` to the given GeoJSON value.
    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.geolocation = Some(geolocation);
    }

    // Getters
    /// Returns `core:sample_start`, the index of this segment's first sample.
    pub fn get_sample_start(&self) -> Integer {
        self.sample_start
    }

    /// Returns `core:datetime`, if set.
    pub fn get_datetime(&self) -> Option<&String> {
        self.datetime.as_ref()
    }

    /// Returns `core:frequency` in Hz, if set.
    pub fn get_frequency(&self) -> Option<&Number> {
        self.frequency.as_ref()
    }

    /// Returns `core:global_index`, if set.
    pub fn get_global_index(&self) -> Option<&Integer> {
        self.global_index.as_ref()
    }

    /// Returns `core:header_bytes`, if set.
    pub fn get_header_bytes(&self) -> Option<&Integer> {
        self.header_bytes.as_ref()
    }
}
