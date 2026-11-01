//! The SigMF `global` object.
//!
//! Every SigMF metadata file contains exactly one `global` object describing
//! properties that apply to the whole recording: the specification version,
//! the sample [`Datatype`], the sample rate, provenance information such as
//! author and hardware, and the list of extensions in use. This module models
//! that object as [`Global`], and the entries of its `core:extensions` array
//! as [`ExtensionDeclaration`].
//!
//! Keys in the `core` namespace map to typed fields on [`Global`]; any other
//! keys (for example, fields defined by extensions) are preserved verbatim and
//! exposed through [`Global::extra_fields`].

use super::{Integer, Number, SigmfError};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Re-exported datatype types used by [`Global::datatype`].
pub use crate::datatype::{ByteType, ComponentType, Datatype, Endianness, SampleDomain};

/// The SigMF `global` object of a metadata file.
///
/// Required fields (`core:version` and `core:datatype`) are stored directly;
/// optional `core` fields are stored as [`Option`]s and are omitted from
/// serialized JSON when `None`. Unknown keys encountered during
/// deserialization (typically extension fields) are retained and serialized
/// back out unchanged; see [`Global::extra_fields`] and
/// [`Global::insert_extra`].
///
/// # Examples
///
/// ```rust
/// use sigmf::global::{Datatype, Global};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let datatype: Datatype = "cf32_le".parse()?;
/// let mut global = Global::new("1.2.6", datatype);
/// global.set_sample_rate(1_000_000.0);
/// global.set_description("Example capture".to_string());
///
/// let json = serde_json::to_value(&global)?;
/// assert_eq!(json["core:version"], "1.2.6");
/// assert_eq!(json["core:datatype"], "cf32_le");
/// assert_eq!(json["core:sample_rate"], 1_000_000.0);
/// assert!(json.get("core:author").is_none());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Global {
    // Required fields
    /// Version of the SigMF specification the metadata conforms to (`core:version`).
    #[serde(rename = "core:version")]
    pub version: String,
    /// Format of the samples stored in the dataset file (`core:datatype`).
    #[serde(rename = "core:datatype")]
    pub datatype: Datatype,

    // Optional fields
    /// Sample rate of the recording in samples per second (`core:sample_rate`).
    #[serde(rename = "core:sample_rate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<Number>,
    /// Name and optionally email address of the recording's author (`core:author`).
    #[serde(rename = "core:author", skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Base filename of the SigMF collection this recording belongs to (`core:collection`).
    #[serde(rename = "core:collection", skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// Digital Object Identifier (DOI) of the dataset file (`core:data_doi`).
    #[serde(rename = "core:data_doi", skip_serializing_if = "Option::is_none")]
    pub data_doi: Option<String>,
    /// Free-text description of the recording (`core:description`).
    #[serde(rename = "core:description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Description of the hardware used to make the recording (`core:hw`).
    #[serde(rename = "core:hw", skip_serializing_if = "Option::is_none")]
    pub hw: Option<String>,
    /// URL of the license under which the recording is distributed (`core:license`).
    #[serde(rename = "core:license", skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Whether the metadata is intentionally distributed without a dataset file
    /// (`core:metadata_only`).
    ///
    /// Must not be used together with Non-Conforming Dataset fields.
    #[serde(rename = "core:metadata_only", skip_serializing_if = "Option::is_none")]
    pub metadata_only: Option<bool>, // Cannot be used in Non-Conforming Datasets
    /// Digital Object Identifier (DOI) of the metadata file (`core:meta_doi`).
    #[serde(rename = "core:meta_doi", skip_serializing_if = "Option::is_none")]
    pub meta_doi: Option<String>,
    /// Number of interleaved channels in the dataset file (`core:num_channels`).
    ///
    /// The SigMF specification treats an absent value as one channel.
    #[serde(rename = "core:num_channels", skip_serializing_if = "Option::is_none")]
    pub num_channels: Option<Integer>,
    /// Index offset of the first sample in the dataset file (`core:offset`).
    #[serde(rename = "core:offset", skip_serializing_if = "Option::is_none")]
    pub offset: Option<Integer>,
    /// Name of the software used to make the recording (`core:recorder`).
    #[serde(rename = "core:recorder", skip_serializing_if = "Option::is_none")]
    pub recorder: Option<String>,
    /// Hex-encoded SHA-512 hash of the dataset file (`core:sha512`).
    #[serde(rename = "core:sha512", skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    /// GeoJSON location of the recording system (`core:geolocation`).
    #[serde(rename = "core:geolocation", skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<GeoJson>,
    /// Extensions used by this recording's metadata (`core:extensions`).
    #[serde(rename = "core:extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<ExtensionDeclaration>>,

    // For Non-Conforming Datasets ONLY
    /// Full filename of a Non-Conforming Dataset file (`core:dataset`).
    #[serde(rename = "core:dataset", skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    /// Number of bytes to ignore at the end of a Non-Conforming Dataset file
    /// (`core:trailing_bytes`).
    #[serde(
        rename = "core:trailing_bytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub trailing_bytes: Option<Integer>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Global {
    /// Creates a [`Global`] with the required `core:version` and
    /// `core:datatype` fields set and every optional field unset.
    ///
    /// No validation of `version` is performed here.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::global::{ComponentType, Datatype, Endianness, Global, SampleDomain};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let datatype = Datatype::new(
    ///     SampleDomain::Complex,
    ///     Some(ComponentType::I16),
    ///     Some(Endianness::LE),
    ///     None,
    /// )?;
    /// let global = Global::new("1.2.6", datatype);
    /// assert_eq!(global.get_version(), "1.2.6");
    /// assert_eq!(global.get_datatype().as_str(), "ci16_le");
    /// assert!(global.get_sample_rate().is_none());
    /// assert!(global.extra_fields().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(version: impl Into<String>, datatype: Datatype) -> Self {
        Self {
            version: version.into(),
            datatype,
            sample_rate: None,
            author: None,
            collection: None,
            data_doi: None,
            description: None,
            hw: None,
            license: None,
            metadata_only: None,
            meta_doi: None,
            num_channels: None,
            offset: None,
            recorder: None,
            sha512: None,
            geolocation: None,
            extensions: None,
            dataset: None,
            trailing_bytes: None,
            extra_fields: Map::new(),
        }
    }

    /// Returns the fields of this object that are not modeled by a typed field,
    /// keyed by their full JSON key.
    ///
    /// These are the keys that did not map to a typed field during
    /// deserialization (typically extension fields such as `antenna:gain`), plus
    /// any added via [`Global::insert_extra`]. They are serialized alongside the
    /// typed `core` fields.
    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn extension_fields_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extra_fields
    }

    pub(crate) fn update_extension_fields(
        &mut self,
        known_fields: &[&str],
        fields: Map<String, Value>,
    ) {
        for field in known_fields {
            self.extra_fields.remove(*field);
        }
        self.extra_fields.extend(fields);
    }

    /// Inserts or replaces an extra (non-typed) field, returning the previous value for
    /// `key` if one was present.
    ///
    /// The key is stored as given; namespace syntax and extension declarations
    /// are not checked here (see [`crate::validation`]).
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `key` is one of the
    /// `core:` global keys modeled by a typed field on [`Global`] (for example
    /// `core:sample_rate`); set those through the corresponding field or setter
    /// instead.
    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, SigmfError> {
        let key = key.into();
        if is_core_global_key(&key) {
            return Err(SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    // Setters
    /// Sets the SigMF specification version (`core:version`).
    pub fn set_version(&mut self, version: String) {
        self.version = version;
    }

    /// Sets the sample format of the dataset (`core:datatype`).
    pub fn set_datatype(&mut self, datatype: Datatype) {
        self.datatype = datatype;
    }

    /// Sets the sample rate in samples per second (`core:sample_rate`).
    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.sample_rate = Some(sample_rate);
    }

    /// Sets the recording author (`core:author`).
    pub fn set_author(&mut self, author: String) {
        self.author = Some(author);
    }

    /// Sets the base filename of the associated collection (`core:collection`).
    pub fn set_collection(&mut self, collection_str: String) {
        self.collection = Some(collection_str);
    }

    /// Sets the DOI of the dataset file (`core:data_doi`).
    pub fn set_data_doi(&mut self, data_doi: String) {
        self.data_doi = Some(data_doi);
    }

    /// Sets the free-text description (`core:description`).
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Sets the hardware description (`core:hw`).
    pub fn set_hw(&mut self, hw: String) {
        self.hw = Some(hw);
    }

    /// Sets the license URL (`core:license`).
    pub fn set_license(&mut self, license: String) {
        self.license = Some(license);
    }

    /// Sets whether the metadata is distributed without a dataset (`core:metadata_only`).
    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.metadata_only = Some(metadata_only);
    }

    /// Sets the DOI of the metadata file (`core:meta_doi`).
    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.meta_doi = Some(meta_doi);
    }

    /// Sets the number of interleaved channels (`core:num_channels`).
    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.num_channels = Some(num_channels);
    }

    /// Sets the index offset of the first sample (`core:offset`).
    pub fn set_offset(&mut self, offset: Integer) {
        self.offset = Some(offset);
    }

    /// Sets the name of the recording software (`core:recorder`).
    pub fn set_recorder(&mut self, recorder: String) {
        self.recorder = Some(recorder);
    }

    /// Sets the SHA-512 hash of the dataset file (`core:sha512`).
    ///
    /// The value is stored as given; it is not computed or verified here.
    pub fn set_sha512(&mut self, hash: String) {
        self.sha512 = Some(hash);
    }

    /// Sets the GeoJSON location of the recording system (`core:geolocation`).
    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.geolocation = Some(geolocation);
    }

    /// Replaces the list of declared extensions (`core:extensions`).
    pub fn set_extensions(&mut self, extensions: Vec<ExtensionDeclaration>) {
        self.extensions = Some(extensions);
    }

    /// Sets the Non-Conforming Dataset filename (`core:dataset`).
    pub fn set_dataset(&mut self, dataset: String) {
        self.dataset = Some(dataset);
    }

    /// Sets the number of trailing bytes to ignore in a Non-Conforming Dataset
    /// (`core:trailing_bytes`).
    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.trailing_bytes = Some(trailing_bytes);
    }

    // Getters
    /// Returns a copy of the SigMF specification version (`core:version`).
    pub fn get_version(&self) -> String {
        self.version.clone()
    }

    /// Returns a copy of the dataset sample format (`core:datatype`).
    pub fn get_datatype(&self) -> Datatype {
        self.datatype.clone()
    }

    /// Returns the sample rate in samples per second (`core:sample_rate`), if set.
    pub fn get_sample_rate(&self) -> Option<Number> {
        self.sample_rate
    }

    /// Returns a copy of the recording author (`core:author`), if set.
    pub fn get_author(&self) -> Option<String> {
        self.author.clone()
    }

    /// Returns a copy of the associated collection's base filename (`core:collection`), if set.
    pub fn get_collection(&self) -> Option<String> {
        self.collection.clone()
    }

    /// Returns a copy of the dataset DOI (`core:data_doi`), if set.
    pub fn get_data_doi(&self) -> Option<String> {
        self.data_doi.clone()
    }

    /// Returns a copy of the free-text description (`core:description`), if set.
    pub fn get_description(&self) -> Option<String> {
        self.description.clone()
    }

    /// Returns a copy of the hardware description (`core:hw`), if set.
    pub fn get_hw(&self) -> Option<String> {
        self.hw.clone()
    }

    /// Returns a copy of the license URL (`core:license`), if set.
    pub fn get_license(&self) -> Option<String> {
        self.license.clone()
    }

    /// Returns the metadata-only flag (`core:metadata_only`), if set.
    pub fn get_metadata_only(&self) -> Option<bool> {
        self.metadata_only
    }

    /// Returns a copy of the metadata DOI (`core:meta_doi`), if set.
    pub fn get_meta_doi(&self) -> Option<String> {
        self.meta_doi.clone()
    }

    /// Returns the number of interleaved channels (`core:num_channels`), if set.
    ///
    /// This returns `None` when the key is absent; it does not apply the
    /// specification's default of one channel.
    pub fn get_num_channels(&self) -> Option<Integer> {
        self.num_channels
    }

    /// Returns the index offset of the first sample (`core:offset`), if set.
    pub fn get_offset(&self) -> Option<Integer> {
        self.offset
    }

    /// Returns a copy of the recording software name (`core:recorder`), if set.
    pub fn get_recorder(&self) -> Option<String> {
        self.recorder.clone()
    }

    /// Returns a copy of the dataset SHA-512 hash (`core:sha512`), if set.
    pub fn get_sha512(&self) -> Option<String> {
        self.sha512.clone()
    }

    /// Returns a copy of the GeoJSON location (`core:geolocation`), if set.
    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.geolocation.clone()
    }

    /// Returns a copy of the declared extensions (`core:extensions`), if set.
    pub fn get_extensions(&self) -> Option<Vec<ExtensionDeclaration>> {
        self.extensions.clone()
    }

    /// Returns a copy of the Non-Conforming Dataset filename (`core:dataset`), if set.
    pub fn get_dataset(&self) -> Option<String> {
        self.dataset.clone()
    }

    /// Returns the number of trailing bytes to ignore (`core:trailing_bytes`), if set.
    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.trailing_bytes
    }
}

fn is_core_global_key(key: &str) -> bool {
    matches!(
        key,
        "core:version"
            | "core:datatype"
            | "core:sample_rate"
            | "core:author"
            | "core:collection"
            | "core:data_doi"
            | "core:description"
            | "core:hw"
            | "core:license"
            | "core:metadata_only"
            | "core:meta_doi"
            | "core:num_channels"
            | "core:offset"
            | "core:recorder"
            | "core:sha512"
            | "core:geolocation"
            | "core:extensions"
            | "core:dataset"
            | "core:trailing_bytes"
    )
}

/// An entry in the `core:extensions` array declaring a SigMF extension.
///
/// Each extension whose namespace is used in a metadata file should be
/// declared so readers know which extension specification (and version) to
/// apply and whether they may safely ignore it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtensionDeclaration {
    /// Extension namespace name, e.g. `antenna` (`name`).
    pub name: String,
    /// Version of the extension specification in use (`version`).
    pub version: String,
    /// Whether the extension may be ignored by readers that do not support it
    /// (`optional`); `false` means the extension is required to interpret the
    /// recording correctly.
    pub optional: bool,
}
