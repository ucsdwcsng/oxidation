//! SigMF recording metadata: the contents of a `.sigmf-meta` file.
//!
//! A SigMF metadata document is a JSON object with three required top-level
//! keys, each modeled by its own type:
//!
//! * `global` — recording-wide properties such as `core:datatype`,
//!   `core:sample_rate`, and `core:version` ([`crate::global::Global`]);
//! * `captures` — segments of the dataset keyed by `core:sample_start`
//!   ([`crate::capture::Capture`]);
//! * `annotations` — labeled sample and frequency ranges
//!   ([`crate::annotation::Annotation`]).
//!
//! [`Metadata`] ties these together, parses and serializes the document
//! (preserving unknown extension keys), offers range queries over captures
//! and annotations, and writes validated metadata to disk. [`MetadataEdit`]
//! provides transactional, validate-then-commit editing, and
//! [`MetadataChange`] describes differences reported by [`Metadata::diff`].

use super::{Integer, Number, annotation, capture, global, utils::hash::hash_from_slice};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha512};
use std::io::Read;

/// A complete SigMF metadata document (`.sigmf-meta`).
///
/// Holds the `global` object, the `captures` and `annotations` arrays, and any
/// unrecognized top-level keys. When parsed with [`Metadata::from_slice`] (or
/// [`from_reader`](Metadata::from_reader) / [`from_path`](Metadata::from_path))
/// the exact input bytes are retained and exposed through
/// [`Metadata::original_bytes`]; any mutation through this type's methods
/// discards them.
///
/// # Examples
///
/// ```rust
/// use sigmf::capture::Capture;
/// use sigmf::global::{Datatype, Global};
/// use sigmf::meta::Metadata;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let global = Global::new("1.2.6", "cf32_le".parse::<Datatype>()?);
/// let mut metadata = Metadata::new(global, Vec::new(), vec![Capture::new(0)]);
/// metadata.set_sample_rate(1_000_000.0);
///
/// let bytes = metadata.to_vec()?;
/// let parsed = Metadata::from_slice(&bytes)?;
/// assert_eq!(parsed.get_sample_rate(), Some(1_000_000.0));
/// assert_eq!(parsed.captures().len(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    global: global::Global,
    annotations: Vec<annotation::Annotation>,
    captures: Vec<capture::Capture>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
    #[serde(skip)]
    original_bytes: Option<Vec<u8>>,
}

impl Metadata {
    /// Creates a metadata document from its `global` object and its
    /// `annotations` and `captures` arrays.
    ///
    /// The document has no top-level extension fields and no retained
    /// original bytes. No validation is performed; see
    /// [`Metadata::validate`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::capture::Capture;
    /// use sigmf::global::{Datatype, Global};
    /// use sigmf::meta::Metadata;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let global = Global::new("1.2.6", "ri16_le".parse::<Datatype>()?);
    /// let metadata = Metadata::new(global, Vec::new(), vec![Capture::new(0)]);
    /// assert_eq!(metadata.get_version(), "1.2.6");
    /// assert!(metadata.annotations().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        global: global::Global,
        annotations: Vec<annotation::Annotation>,
        captures: Vec<capture::Capture>,
    ) -> Self {
        Metadata {
            global,
            annotations,
            captures,
            extra_fields: Map::new(),
            original_bytes: None,
        }
    }
    // ================================== Methods ================================== //

    /// Parses a metadata document from JSON bytes.
    ///
    /// Before deserializing, the input is checked for duplicate object keys
    /// and for the basic shape required by SigMF: a `global` object with
    /// string `core:version` and `core:datatype`, `captures` and
    /// `annotations` arrays whose entries are objects with a non-negative
    /// integer `core:sample_start`, and no explicit `null` for optional core
    /// fields. Unknown keys are preserved. The input bytes are retained and
    /// available via [`Metadata::original_bytes`].
    ///
    /// This does not run full semantic validation; see
    /// [`Metadata::validate`].
    ///
    /// # Errors
    ///
    /// * [`crate::SigmfError::ResourceLimit`] if `bytes` exceeds the 1 MB
    ///   metadata size limit.
    /// * [`crate::SigmfError::InvalidJson`] if the input is not valid JSON or
    ///   an object contains a duplicate key.
    /// * [`crate::SigmfError::InvalidMetadata`] if a required field is
    ///   missing, has the wrong type, an optional core field is `null`, or the
    ///   document otherwise fails to deserialize (including an invalid
    ///   `core:datatype`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::meta::Metadata;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let json = br#"{
    ///     "global": {"core:version": "1.2.6", "core:datatype": "cf32_le"},
    ///     "captures": [{"core:sample_start": 0, "core:frequency": 915e6}],
    ///     "annotations": []
    /// }"#;
    /// let metadata = Metadata::from_slice(json)?;
    /// assert_eq!(metadata.get_datatype().to_string(), "cf32_le");
    /// assert_eq!(metadata.captures()[0].frequency, Some(915e6));
    /// assert_eq!(metadata.original_bytes(), Some(&json[..]));
    ///
    /// assert!(Metadata::from_slice(br#"{"global": {}}"#).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice(bytes: &[u8]) -> Result<Self, crate::SigmfError> {
        if bytes.len() > crate::MAX_META_SIZE {
            return Err(crate::SigmfError::ResourceLimit(format!(
                "metadata exceeds the {} byte limit",
                crate::MAX_META_SIZE
            )));
        }
        let value = crate::json::parse_unique_value(bytes)?;
        Self::validate_metadata_shape(&value)?;
        let mut metadata: Self = serde_json::from_value(value)
            .map_err(|error| crate::SigmfError::InvalidMetadata(error.to_string()))?;
        metadata.original_bytes = Some(bytes.to_vec());
        Ok(metadata)
    }

    /// Serializes the document as compact JSON.
    ///
    /// Optional fields that are unset are omitted, and extension fields are
    /// written back alongside the core fields. The output is freshly
    /// serialized and is not guaranteed to match [`Metadata::original_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::Serde`] if serialization fails.
    pub fn to_vec(&self) -> Result<Vec<u8>, crate::SigmfError> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Serializes the document as pretty-printed (indented) JSON.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::Serde`] if serialization fails.
    pub fn to_vec_pretty(&self) -> Result<Vec<u8>, crate::SigmfError> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    /// Validates the document under `policy`, then writes its compact JSON
    /// serialization to `writer`.
    ///
    /// Returns the SHA-512 digest (lowercase hex) of the exact bytes written,
    /// suitable for a collection's `core:streams` `hash` entry. Validation
    /// warnings do not prevent writing.
    ///
    /// # Errors
    ///
    /// * [`crate::SigmfError::InvalidMetadata`] if validation fails; nothing
    ///   is written in that case.
    /// * [`crate::SigmfError::Serde`] if serialization fails.
    /// * [`crate::SigmfError::Io`] if writing to `writer` fails.
    pub fn write_to(
        &self,
        mut writer: impl std::io::Write,
        policy: crate::validation::ValidationPolicy,
    ) -> Result<String, crate::SigmfError> {
        let report = self.validate(policy);
        if report.status == crate::validation::ValidationStatus::Failed {
            return Err(crate::SigmfError::InvalidMetadata(
                "metadata failed validation and was not serialized".to_string(),
            ));
        }
        let bytes = self.to_vec()?;
        writer.write_all(&bytes)?;
        hash_from_slice(&bytes)
    }

    /// Validates the document under `policy` and atomically writes it to a new
    /// file at `path`.
    ///
    /// The JSON is written to a temporary sibling file, synced, and then
    /// published under `path` without overwriting an existing file. Returns
    /// the SHA-512 digest (lowercase hex) of the bytes written.
    ///
    /// # Errors
    ///
    /// * [`crate::SigmfError::InvalidMetadata`] if validation fails or `path`
    ///   does not name a file.
    /// * [`crate::SigmfError::Serde`] if serialization fails.
    /// * [`crate::SigmfError::Io`] if `path` already exists or any file
    ///   system operation fails.
    pub fn save_new(
        &self,
        path: impl AsRef<std::path::Path>,
        policy: crate::validation::ValidationPolicy,
    ) -> Result<String, crate::SigmfError> {
        self.save_atomic(path.as_ref(), policy, false)
    }

    /// Validates the document under `policy` and atomically writes it to
    /// `path`, replacing any existing file.
    ///
    /// The JSON is written to a temporary sibling file, synced, and renamed
    /// over `path`, so readers observe either the old or the new contents.
    /// Returns the SHA-512 digest (lowercase hex) of the bytes written.
    ///
    /// # Errors
    ///
    /// * [`crate::SigmfError::InvalidMetadata`] if validation fails or `path`
    ///   does not name a file.
    /// * [`crate::SigmfError::Serde`] if serialization fails.
    /// * [`crate::SigmfError::Io`] if any file system operation fails.
    pub fn replace_atomic(
        &self,
        path: impl AsRef<std::path::Path>,
        policy: crate::validation::ValidationPolicy,
    ) -> Result<String, crate::SigmfError> {
        self.save_atomic(path.as_ref(), policy, true)
    }

    fn save_atomic(
        &self,
        path: &std::path::Path,
        policy: crate::validation::ValidationPolicy,
        replace: bool,
    ) -> Result<String, crate::SigmfError> {
        let report = self.validate(policy);
        if report.status == crate::validation::ValidationStatus::Failed {
            return Err(crate::SigmfError::InvalidMetadata(
                "metadata failed validation and was not published".to_string(),
            ));
        }
        let bytes = self.to_vec()?;
        let digest = hash_from_slice(&bytes)?;
        crate::persistence::write_atomic(path, &bytes, replace)?;
        Ok(digest)
    }

    /// Reads and parses a metadata document from `reader`.
    ///
    /// At most 1 MB plus one byte is read; the bytes are then parsed with
    /// [`Metadata::from_slice`].
    ///
    /// # Errors
    ///
    /// * [`crate::SigmfError::Io`] if reading fails.
    /// * [`crate::SigmfError::InvalidMetadata`] if the input exceeds the 1 MB
    ///   metadata size limit.
    /// * Any error returned by [`Metadata::from_slice`].
    pub fn from_reader(reader: impl std::io::Read) -> Result<Self, crate::SigmfError> {
        let mut bytes = Vec::new();
        reader
            .take(crate::MAX_META_SIZE as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > crate::MAX_META_SIZE {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "metadata exceeds the {} byte limit",
                crate::MAX_META_SIZE
            )));
        }
        Self::from_slice(&bytes)
    }

    /// Opens the `.sigmf-meta` file at `path` and parses it with
    /// [`Metadata::from_reader`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::Io`] if the file cannot be opened, or any
    /// error returned by [`Metadata::from_reader`].
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, crate::SigmfError> {
        Self::from_reader(std::fs::File::open(path)?)
    }

    /// Returns the exact bytes this document was parsed from, if it was
    /// parsed and has not been modified since.
    ///
    /// Returns `None` for documents built with [`Metadata::new`] and after
    /// calling a mutating method on [`Metadata`] (setters, `insert_*`, or
    /// `add_*`).
    pub fn original_bytes(&self) -> Option<&[u8]> {
        self.original_bytes.as_deref()
    }

    /// Returns the SHA-512 digest (lowercase hex) of
    /// [`Metadata::original_bytes`], or `None` if no original bytes are
    /// retained.
    ///
    /// This is the hash a collection's `core:streams` entry should record for
    /// an unmodified, previously parsed metadata file.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::SigmfError`] if hashing fails (currently
    /// infallible).
    pub fn original_digest(&self) -> Result<Option<String>, crate::SigmfError> {
        self.original_bytes
            .as_deref()
            .map(hash_from_slice)
            .transpose()
    }

    /// Returns the SHA-512 digest (lowercase hex) of the document's compact
    /// serialization, as produced by [`Metadata::to_vec`].
    ///
    /// This may differ from [`Metadata::original_digest`] even for an
    /// unmodified document, because re-serialization does not preserve the
    /// original formatting.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::Serde`] if serialization fails.
    pub fn hash(&self) -> Result<String, crate::SigmfError> {
        let serialized = self.to_vec()?;
        hash_from_slice(&serialized)
    }

    fn revision_fingerprint(&self) -> String {
        let digest = Sha512::digest(format!("{self:?}").as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Returns the `global` object.
    pub fn global(&self) -> &global::Global {
        &self.global
    }

    /// Returns the `captures` array in document order.
    pub fn captures(&self) -> &[capture::Capture] {
        &self.captures
    }

    /// Returns the `annotations` array in document order.
    pub fn annotations(&self) -> &[annotation::Annotation] {
        &self.annotations
    }

    /// Returns the unrecognized top-level keys (anything other than `global`,
    /// `captures`, and `annotations`).
    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    /// Returns the capture segment that contains `sample_index`.
    ///
    /// This is the capture with the greatest `core:sample_start` that is less
    /// than or equal to `sample_index`, regardless of the order of the
    /// `captures` array. Returns `None` if every capture starts after
    /// `sample_index` or there are no captures. If several captures share
    /// that start, the last one in document order is returned.
    pub fn capture_at(&self, sample_index: Integer) -> Option<&capture::Capture> {
        self.captures
            .iter()
            .filter(|capture| capture.sample_start <= sample_index)
            .max_by_key(|capture| capture.sample_start)
    }

    /// Returns the annotations whose sample range overlaps the half-open
    /// range `[start, start + sample_count)`.
    ///
    /// An annotation's range is `[core:sample_start, core:sample_start +
    /// core:sample_count)`. Annotations without `core:sample_count`, or with
    /// a count of zero, never match. An empty query range returns no
    /// annotations. Results are in document order.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidRange`] if `start + sample_count`
    /// overflows, or if a candidate annotation's end overflows.
    pub fn annotations_intersecting_samples(
        &self,
        start: Integer,
        sample_count: Integer,
    ) -> Result<Vec<&annotation::Annotation>, crate::SigmfError> {
        let end = start
            .checked_add(sample_count)
            .ok_or_else(|| crate::SigmfError::InvalidRange("sample range end overflows".into()))?;
        if sample_count == 0 {
            return Ok(Vec::new());
        }
        let mut matches = Vec::new();
        for annotation in &self.annotations {
            let Some(count) = annotation.sample_count else {
                continue;
            };
            if count == 0 {
                continue;
            }
            let annotation_end = annotation.sample_start.checked_add(count).ok_or_else(|| {
                crate::SigmfError::InvalidRange(format!(
                    "annotation at sample {} has an overflowing range",
                    annotation.sample_start
                ))
            })?;
            if annotation.sample_start < end && annotation_end > start {
                matches.push(annotation);
            }
        }
        Ok(matches)
    }

    /// Returns the annotations whose frequency range overlaps the closed
    /// interval `[lower, upper]` (in Hz).
    ///
    /// Only annotations that set both `core:freq_lower_edge` and
    /// `core:freq_upper_edge` are considered; touching edges count as an
    /// overlap. Results are in document order.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidRange`] if either bound is not
    /// finite or if `lower > upper`.
    pub fn annotations_intersecting_frequency(
        &self,
        lower: Number,
        upper: Number,
    ) -> Result<Vec<&annotation::Annotation>, crate::SigmfError> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err(crate::SigmfError::InvalidRange(
                "frequency interval must be finite and ordered".into(),
            ));
        }
        Ok(self
            .annotations
            .iter()
            .filter(|annotation| {
                matches!(
                    (annotation.freq_lower_edge, annotation.freq_upper_edge),
                    (Some(annotation_lower), Some(annotation_upper))
                        if annotation_lower <= upper && annotation_upper >= lower
                )
            })
            .collect())
    }

    /// Computes the leaf-level differences between this document and
    /// `other`.
    ///
    /// Both documents are serialized to JSON and compared recursively.
    /// Objects are compared key by key and arrays index by index, so
    /// inserting an element shifts every later index. Each reported change
    /// carries an RFC 6901 JSON Pointer; the result is empty when the
    /// serializations are equal.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::Serde`] if either document fails to
    /// serialize.
    pub fn diff(&self, other: &Self) -> Result<Vec<MetadataChange>, crate::SigmfError> {
        let before = serde_json::to_value(self)?;
        let after = serde_json::to_value(other)?;
        let mut changes = Vec::new();
        collect_changes("", Some(&before), Some(&after), &mut changes);
        Ok(changes)
    }

    /// Inserts or replaces a top-level extension key.
    ///
    /// Returns the previous value under `key`, if any, and clears the
    /// retained [original bytes](Metadata::original_bytes). The key is not
    /// checked against declared extensions.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`] if `key` is `global`,
    /// `captures`, or `annotations`.
    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if matches!(key.as_str(), "global" | "captures" | "annotations") {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        self.original_bytes = None;
        Ok(self.extra_fields.insert(key, value))
    }

    /// Inserts or replaces an extension key in the `global` object.
    ///
    /// Returns the previous value under `key`, if any. Delegates to
    /// [`crate::global::Global::insert_extra`]. The retained
    /// [original bytes](Metadata::original_bytes) are cleared even if the
    /// insertion is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`] if `key` names a core
    /// global field (for example `core:version`).
    pub fn insert_global_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        self.original_bytes = None;
        self.global.insert_extra(key, value)
    }

    pub(crate) fn crop_samples(
        &self,
        sample_start: Integer,
        sample_count: Integer,
        mut extension_handler: Option<&mut dyn crate::extensions::CropExtensionHandler>,
    ) -> Result<Self, crate::SigmfError> {
        let crop_end = sample_start
            .checked_add(sample_count)
            .ok_or_else(|| crate::SigmfError::InvalidRange("sample range end overflows".into()))?;
        if sample_start < self.global.offset.unwrap_or(0) {
            return Err(crate::SigmfError::InvalidRange(format!(
                "crop start {sample_start} precedes dataset offset {}",
                self.global.offset.unwrap_or(0)
            )));
        }
        if !self.extra_fields.is_empty() {
            return Err(crate::SigmfError::InvalidMetadata(
                "sample crop cannot safely transform top-level extension fields".into(),
            ));
        }

        let mut cropped = self.clone();
        transform_crop_extension_fields(
            &mut extension_handler,
            crate::extensions::ExtensionScope::Global,
            None,
            None,
            sample_start,
            sample_count,
            cropped.global.extension_fields_mut(),
        )?;

        let previous_capture = if sample_count == 0 {
            None
        } else {
            self.captures
                .iter()
                .enumerate()
                .filter(|(_, capture)| capture.sample_start <= sample_start)
                .max_by_key(|(_, capture)| capture.sample_start)
                .map(|(index, _)| index)
        };
        cropped.captures = self
            .captures
            .iter()
            .enumerate()
            .filter(|(index, capture)| {
                sample_count > 0
                    && (Some(*index) == previous_capture
                        || (capture.sample_start >= sample_start
                            && capture.sample_start < crop_end))
            })
            .map(|(_, capture)| {
                let mut capture = capture.clone();
                transform_crop_extension_fields(
                    &mut extension_handler,
                    crate::extensions::ExtensionScope::Capture,
                    Some(capture.sample_start),
                    None,
                    sample_start,
                    sample_count,
                    capture.extension_fields_mut(),
                )?;
                capture.header_bytes = None;
                Ok(capture)
            })
            .collect::<Result<Vec<_>, crate::SigmfError>>()?;

        cropped.annotations = Vec::new();
        if sample_count > 0 {
            for annotation in &self.annotations {
                let (cropped_start, cropped_count) = match annotation.sample_count {
                    Some(count) if count > 0 => {
                        let annotation_end =
                            annotation.sample_start.checked_add(count).ok_or_else(|| {
                                crate::SigmfError::InvalidRange(format!(
                                    "annotation at sample {} has an overflowing range",
                                    annotation.sample_start
                                ))
                            })?;
                        let start = annotation.sample_start.max(sample_start);
                        let end = annotation_end.min(crop_end);
                        if start >= end {
                            continue;
                        }
                        (start, Some(end - start))
                    }
                    Some(_) => continue,
                    None if annotation.sample_start >= sample_start
                        && annotation.sample_start < crop_end =>
                    {
                        (annotation.sample_start, None)
                    }
                    None => continue,
                };
                let mut annotation = annotation.clone();
                transform_crop_extension_fields(
                    &mut extension_handler,
                    crate::extensions::ExtensionScope::Annotation,
                    Some(annotation.sample_start),
                    annotation.sample_count,
                    sample_start,
                    sample_count,
                    annotation.extension_fields_mut(),
                )?;
                annotation.sample_start = cropped_start;
                annotation.sample_count = cropped_count;
                cropped.annotations.push(annotation);
            }
        }

        cropped.global.offset = Some(sample_start);
        cropped.global.sha512 = None;
        cropped.global.dataset = None;
        cropped.global.trailing_bytes = None;
        cropped.original_bytes = None;
        Ok(cropped)
    }

    pub(crate) fn update_global_extension_fields(
        &mut self,
        known_fields: &[&str],
        fields: Map<String, Value>,
    ) {
        self.global.update_extension_fields(known_fields, fields);
        self.original_bytes = None;
    }

    /// Starts a transactional edit of this document.
    ///
    /// Changes are made to a draft copy; they are applied to `self` only when
    /// [`MetadataEdit::commit`] succeeds, and discarded if the
    /// [`MetadataEdit`] is dropped.
    pub fn edit(&mut self) -> MetadataEdit<'_> {
        let draft = self.clone();
        MetadataEdit {
            target: self,
            draft,
            validation: None,
        }
    }

    // ================================== Setters ================================== //
    // ---------------- Global ---------------- //
    /// Sets the global `core:version` field: the SigMF specification version
    /// string.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_version(&mut self, version: String) {
        self.original_bytes = None;
        self.global.set_version(version);
    }

    /// Sets the global `core:datatype` field: the sample format of the dataset.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_datatype(&mut self, datatype: global::Datatype) {
        self.original_bytes = None;
        self.global.set_datatype(datatype);
    }

    /// Sets the global `core:sample_rate` field: the sample rate in samples per
    /// second.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.original_bytes = None;
        self.global.set_sample_rate(sample_rate);
    }

    /// Sets the global `core:author` field: the author of the recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_author(&mut self, author: String) {
        self.original_bytes = None;
        self.global.set_author(author);
    }

    /// Sets the global `core:collection` field: the base name of the SigMF
    /// collection this recording belongs to.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_collection(&mut self, collection_str: String) {
        self.original_bytes = None;
        self.global.set_collection(collection_str);
    }

    /// Sets the global `core:data_doi` field: the DOI of the dataset file.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_data_doi(&mut self, data_doi: String) {
        self.original_bytes = None;
        self.global.set_data_doi(data_doi);
    }

    /// Sets the global `core:description` field: the free-text description of
    /// the recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_description(&mut self, description: String) {
        self.original_bytes = None;
        self.global.set_description(description);
    }

    /// Sets the global `core:hw` field: the description of the recording
    /// hardware.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_hw(&mut self, hw: String) {
        self.original_bytes = None;
        self.global.set_hw(hw);
    }

    /// Sets the global `core:license` field: the URL of the license governing
    /// the recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_license(&mut self, license: String) {
        self.original_bytes = None;
        self.global.set_license(license);
    }

    /// Sets the global `core:metadata_only` field: whether the recording
    /// intentionally has no dataset file.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.original_bytes = None;
        self.global.set_metadata_only(metadata_only);
    }

    /// Sets the global `core:meta_doi` field: the DOI of the metadata file.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.original_bytes = None;
        self.global.set_meta_doi(meta_doi);
    }

    /// Sets the global `core:num_channels` field: the number of interleaved
    /// channels in the dataset.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.original_bytes = None;
        self.global.set_num_channels(num_channels);
    }

    /// Sets the global `core:offset` field: the index offset of the first
    /// sample in the dataset.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_offset(&mut self, offset: Integer) {
        self.original_bytes = None;
        self.global.set_offset(offset);
    }

    /// Sets the global `core:recorder` field: the name of the software that
    /// made the recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_recorder(&mut self, recorder: String) {
        self.original_bytes = None;
        self.global.set_recorder(recorder);
    }

    /// Sets the global `core:sha512` field: the SHA-512 digest of the dataset
    /// file.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_sha512(&mut self, hash: String) {
        self.original_bytes = None;
        self.global.set_sha512(hash);
    }

    /// Sets the global `core:geolocation` field: the GeoJSON location of the
    /// recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.original_bytes = None;
        self.global.set_geolocation(geolocation);
    }

    /// Sets the global `core:extensions` field: the list of extension
    /// namespaces used by the recording.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_extensions(&mut self, extensions: Vec<global::ExtensionDeclaration>) {
        self.original_bytes = None;
        self.global.set_extensions(extensions);
    }

    /// Sets the global `core:dataset` field: the file name of a non-conforming
    /// dataset.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_dataset(&mut self, dataset: String) {
        self.original_bytes = None;
        self.global.set_dataset(dataset);
    }

    /// Sets the global `core:trailing_bytes` field: the number of non-sample
    /// bytes at the end of the dataset file.
    ///
    /// Clears the retained [original bytes](Metadata::original_bytes).
    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.original_bytes = None;
        self.global.set_trailing_bytes(trailing_bytes);
    }

    // ---------------- Annotations ---------------- //
    /// Appends an entry to the `annotations` array.
    ///
    /// No ordering is enforced. Clears the retained
    /// [original bytes](Metadata::original_bytes).
    pub fn add_annotation(&mut self, annotation: annotation::Annotation) {
        self.original_bytes = None;
        self.annotations.push(annotation);
    }

    // ----------------- Captures ------------------ //
    /// Appends an entry to the `captures` array.
    ///
    /// No ordering is enforced. Clears the retained
    /// [original bytes](Metadata::original_bytes).
    pub fn add_capture(&mut self, capture: capture::Capture) {
        self.original_bytes = None;
        self.captures.push(capture);
    }

    fn validate_metadata_shape(value: &Value) -> Result<(), crate::SigmfError> {
        let object = value.as_object().ok_or_else(|| {
            crate::SigmfError::InvalidMetadata("expected an object at JSON Pointer /".to_string())
        })?;
        let global = object
            .get("global")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                crate::SigmfError::InvalidMetadata(
                    "expected an object at JSON Pointer /global".to_string(),
                )
            })?;
        for required in ["core:version", "core:datatype"] {
            if !global.contains_key(required) {
                return Err(crate::SigmfError::InvalidMetadata(format!(
                    "missing required field at JSON Pointer /global/{required}"
                )));
            }
        }
        if !global["core:version"].is_string() {
            return Err(crate::SigmfError::InvalidMetadata(
                "expected a string at JSON Pointer /global/core:version".to_string(),
            ));
        }
        if !global["core:datatype"].is_string() {
            return Err(crate::SigmfError::InvalidMetadata(
                "expected a string at JSON Pointer /global/core:datatype".to_string(),
            ));
        }
        Self::reject_null_fields(
            global,
            "/global",
            &[
                "core:sample_rate",
                "core:author",
                "core:collection",
                "core:data_doi",
                "core:description",
                "core:hw",
                "core:license",
                "core:metadata_only",
                "core:meta_doi",
                "core:num_channels",
                "core:offset",
                "core:recorder",
                "core:sha512",
                "core:geolocation",
                "core:extensions",
                "core:dataset",
                "core:trailing_bytes",
            ],
        )?;
        for required in ["captures", "annotations"] {
            if !object.get(required).is_some_and(Value::is_array) {
                return Err(crate::SigmfError::InvalidMetadata(format!(
                    "expected an array at JSON Pointer /{required}"
                )));
            }
        }
        for (name, field) in [
            ("captures", "core:sample_start"),
            ("annotations", "core:sample_start"),
        ] {
            for (index, item) in object[name].as_array().into_iter().flatten().enumerate() {
                let pointer = format!("/{name}/{index}");
                let Some(item) = item.as_object() else {
                    return Err(crate::SigmfError::InvalidMetadata(format!(
                        "expected an object at JSON Pointer {pointer}"
                    )));
                };
                if !item.contains_key(field) {
                    return Err(crate::SigmfError::InvalidMetadata(format!(
                        "missing required field at JSON Pointer /{name}/{index}/{field}"
                    )));
                }
                if !item[field].is_u64() {
                    return Err(crate::SigmfError::InvalidMetadata(format!(
                        "expected a nonnegative integer at JSON Pointer /{name}/{index}/{field}"
                    )));
                }
                let optional_fields = if name == "captures" {
                    &[
                        "core:datetime",
                        "core:frequency",
                        "core:global_index",
                        "core:header_bytes",
                        "core:geolocation",
                    ][..]
                } else {
                    &[
                        "core:sample_count",
                        "core:freq_lower_edge",
                        "core:freq_upper_edge",
                        "core:label",
                        "core:comment",
                        "core:generator",
                        "core:uuid",
                    ][..]
                };
                Self::reject_null_fields(item, &pointer, optional_fields)?;
            }
        }
        Ok(())
    }

    fn reject_null_fields(
        object: &serde_json::Map<String, Value>,
        pointer: &str,
        fields: &[&str],
    ) -> Result<(), crate::SigmfError> {
        for field in fields {
            if object.get(*field).is_some_and(Value::is_null) {
                return Err(crate::SigmfError::InvalidMetadata(format!(
                    "null is not permitted at JSON Pointer {pointer}/{field}; omit the field instead"
                )));
            }
        }
        Ok(())
    }

    // ================================== Getters ================================== //
    // ---------------- Global ---------------- //
    /// Returns the global `core:version` field: the SigMF specification version
    /// string.
    pub fn get_version(&self) -> String {
        self.global.get_version()
    }

    /// Returns the global `core:datatype` field: the sample format of the
    /// dataset.
    pub fn get_datatype(&self) -> global::Datatype {
        self.global.get_datatype()
    }

    /// Returns the global `core:sample_rate` field: the sample rate in samples
    /// per second, if set.
    pub fn get_sample_rate(&self) -> Option<Number> {
        self.global.get_sample_rate()
    }

    /// Returns the global `core:author` field: the author of the recording, if
    /// set.
    pub fn get_author(&self) -> Option<String> {
        self.global.get_author()
    }

    /// Returns the global `core:collection` field: the base name of the SigMF
    /// collection this recording belongs to, if set.
    pub fn get_collection(&self) -> Option<String> {
        self.global.get_collection()
    }

    /// Returns the global `core:data_doi` field: the DOI of the dataset file,
    /// if set.
    pub fn get_data_doi(&self) -> Option<String> {
        self.global.get_data_doi()
    }

    /// Returns the global `core:description` field: the free-text description
    /// of the recording, if set.
    pub fn get_description(&self) -> Option<String> {
        self.global.get_description()
    }

    /// Returns the global `core:hw` field: the description of the recording
    /// hardware, if set.
    pub fn get_hw(&self) -> Option<String> {
        self.global.get_hw()
    }

    /// Returns the global `core:license` field: the URL of the license
    /// governing the recording, if set.
    pub fn get_license(&self) -> Option<String> {
        self.global.get_license()
    }

    /// Returns the global `core:metadata_only` field: whether the recording
    /// intentionally has no dataset file, if set.
    pub fn get_metadata_only(&self) -> Option<bool> {
        self.global.get_metadata_only()
    }

    /// Returns the global `core:meta_doi` field: the DOI of the metadata file,
    /// if set.
    pub fn get_meta_doi(&self) -> Option<String> {
        self.global.get_meta_doi()
    }

    /// Returns the global `core:num_channels` field: the number of interleaved
    /// channels in the dataset, if set.
    pub fn get_num_channels(&self) -> Option<Integer> {
        self.global.get_num_channels()
    }

    /// Returns the global `core:offset` field: the index offset of the first
    /// sample in the dataset, if set.
    pub fn get_offset(&self) -> Option<Integer> {
        self.global.get_offset()
    }

    /// Returns the global `core:recorder` field: the name of the software that
    /// made the recording, if set.
    pub fn get_recorder(&self) -> Option<String> {
        self.global.get_recorder()
    }

    /// Returns the global `core:sha512` field: the SHA-512 digest of the
    /// dataset file, if set.
    pub fn get_sha512(&self) -> Option<String> {
        self.global.get_sha512()
    }

    /// Returns the global `core:geolocation` field: the GeoJSON location of the
    /// recording, if set.
    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.global.get_geolocation()
    }

    /// Returns the global `core:extensions` field: the list of extension
    /// namespaces used by the recording, if set.
    pub fn get_extensions(&self) -> Option<Vec<global::ExtensionDeclaration>> {
        self.global.get_extensions()
    }

    /// Returns the global `core:dataset` field: the file name of a
    /// non-conforming dataset, if set.
    pub fn get_dataset(&self) -> Option<String> {
        self.global.get_dataset()
    }

    /// Returns the global `core:trailing_bytes` field: the number of non-sample
    /// bytes at the end of the dataset file, if set.
    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.global.get_trailing_bytes()
    }

    // ---------------- Annotations ---------------- //
    /// Returns a clone of the `annotations` array.
    ///
    /// Prefer [`Metadata::annotations`] to borrow without cloning.
    pub fn get_annotations(&self) -> Vec<annotation::Annotation> {
        self.annotations.clone()
    }

    // ----------------- Captures ------------------ //
    /// Returns a clone of the `captures` array.
    ///
    /// Prefer [`Metadata::captures`] to borrow without cloning.
    pub fn get_captures(&self) -> Vec<capture::Capture> {
        self.captures.clone()
    }
}

fn transform_crop_extension_fields(
    handler: &mut Option<&mut dyn crate::extensions::CropExtensionHandler>,
    scope: crate::extensions::ExtensionScope,
    sample_start: Option<Integer>,
    sample_count: Option<Integer>,
    crop_start: Integer,
    crop_count: Integer,
    fields: &mut Map<String, Value>,
) -> Result<(), crate::SigmfError> {
    if fields.is_empty() {
        return Ok(());
    }
    let Some(handler) = handler.as_mut() else {
        return Err(crate::SigmfError::InvalidMetadata(format!(
            "sample crop requires an extension handler for {scope:?} extension fields"
        )));
    };
    (**handler).transform_for_crop(
        scope,
        sample_start,
        sample_count,
        crop_start,
        crop_count,
        fields,
    )
}

/// A single leaf-level difference reported by [`Metadata::diff`].
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataChange {
    /// RFC 6901 JSON Pointer to the changed value (for example
    /// `/global/core:description`), or `/` for the document root.
    pub json_pointer: String,
    /// The value in the original document, or `None` if it was added.
    pub before: Option<Value>,
    /// The value in the other document, or `None` if it was removed.
    pub after: Option<Value>,
}

fn collect_changes(
    pointer: &str,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<MetadataChange>,
) {
    if before == after {
        return;
    }
    if let (Some(Value::Object(before)), Some(Value::Object(after))) = (before, after) {
        let keys = before
            .keys()
            .chain(after.keys())
            .collect::<std::collections::BTreeSet<_>>();
        for key in keys {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            let child_pointer = format!("{pointer}/{escaped}");
            collect_changes(&child_pointer, before.get(key), after.get(key), changes);
        }
        return;
    }
    if let (Some(Value::Array(before)), Some(Value::Array(after))) = (before, after) {
        for index in 0..before.len().max(after.len()) {
            let child_pointer = format!("{pointer}/{index}");
            collect_changes(&child_pointer, before.get(index), after.get(index), changes);
        }
        return;
    }
    changes.push(MetadataChange {
        json_pointer: if pointer.is_empty() {
            "/".to_string()
        } else {
            pointer.to_string()
        },
        before: before.cloned(),
        after: after.cloned(),
    });
}

/// A transactional edit of a [`Metadata`] document, created by
/// [`Metadata::edit`].
///
/// Edits are applied to a draft. To publish them, call
/// [`MetadataEdit::validate`] and then [`MetadataEdit::commit`]; any change
/// made after validating invalidates the report, so `commit` always applies
/// exactly the draft that was validated. Dropping the edit discards the
/// draft.
///
/// # Examples
///
/// ```rust
/// use sigmf::meta::Metadata;
/// use sigmf::validation::ValidationPolicy;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut metadata = Metadata::from_slice(br#"{
///     "global": {"core:version": "1.2.6", "core:datatype": "ri16_le"},
///     "captures": [],
///     "annotations": []
/// }"#)?;
///
/// let mut edit = metadata.edit();
/// edit.set_description("reviewed".to_string());
/// edit.validate(ValidationPolicy::StrictSupportedVersion);
/// edit.commit()?;
///
/// assert_eq!(metadata.get_description().as_deref(), Some("reviewed"));
/// # Ok(())
/// # }
/// ```
pub struct MetadataEdit<'a> {
    target: &'a mut Metadata,
    draft: Metadata,
    validation: Option<crate::validation::ValidationReport>,
}

impl MetadataEdit<'_> {
    /// Returns the current draft.
    pub fn metadata(&self) -> &Metadata {
        &self.draft
    }

    /// Returns mutable access to the draft for edits not covered by the
    /// convenience methods.
    ///
    /// Discards any validation report, so [`MetadataEdit::validate`] must be
    /// called again before committing.
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        self.validation = None;
        self.draft.original_bytes = None;
        &mut self.draft
    }

    /// Sets the draft's global `core:description` field and discards any
    /// validation report.
    pub fn set_description(&mut self, description: String) {
        self.draft.set_description(description);
        self.validation = None;
    }

    /// Sets the draft's global `core:datatype` field and discards any
    /// validation report.
    pub fn set_datatype(&mut self, datatype: global::Datatype) {
        self.draft.set_datatype(datatype);
        self.validation = None;
    }

    /// Sets the draft's global `core:offset` field and discards any
    /// validation report.
    pub fn set_offset(&mut self, offset: Integer) {
        self.draft.set_offset(offset);
        self.validation = None;
    }

    /// Appends an annotation to the draft and discards any validation report.
    pub fn add_annotation(&mut self, annotation: annotation::Annotation) {
        self.draft.add_annotation(annotation);
        self.validation = None;
    }

    /// Replaces the draft annotation at `index`, returning the old one.
    ///
    /// Returns `None` and leaves the draft (and any validation report)
    /// unchanged if `index` is out of bounds.
    pub fn replace_annotation(
        &mut self,
        index: usize,
        annotation: annotation::Annotation,
    ) -> Option<annotation::Annotation> {
        let current = self.draft.annotations.get_mut(index)?;
        self.validation = None;
        Some(std::mem::replace(current, annotation))
    }

    /// Appends a capture to the draft and discards any validation report.
    pub fn add_capture(&mut self, capture: capture::Capture) {
        self.draft.add_capture(capture);
        self.validation = None;
    }

    /// Replaces the draft capture at `index`, returning the old one.
    ///
    /// Returns `None` and leaves the draft (and any validation report)
    /// unchanged if `index` is out of bounds.
    pub fn replace_capture(
        &mut self,
        index: usize,
        capture: capture::Capture,
    ) -> Option<capture::Capture> {
        let current = self.draft.captures.get_mut(index)?;
        self.validation = None;
        Some(std::mem::replace(current, capture))
    }

    /// Removes and returns the draft annotation at `index`, shifting later
    /// annotations down.
    ///
    /// Returns `None` and leaves the draft (and any validation report)
    /// unchanged if `index` is out of bounds.
    pub fn remove_annotation(&mut self, index: usize) -> Option<annotation::Annotation> {
        if index < self.draft.annotations.len() {
            self.validation = None;
            Some(self.draft.annotations.remove(index))
        } else {
            None
        }
    }

    /// Removes and returns the draft capture at `index`, shifting later
    /// captures down.
    ///
    /// Returns `None` and leaves the draft (and any validation report)
    /// unchanged if `index` is out of bounds.
    pub fn remove_capture(&mut self, index: usize) -> Option<capture::Capture> {
        if index < self.draft.captures.len() {
            self.validation = None;
            Some(self.draft.captures.remove(index))
        } else {
            None
        }
    }

    /// Stably sorts the draft's captures and annotations by
    /// `core:sample_start`, as SigMF requires, and discards any validation
    /// report.
    ///
    /// Entries with equal `core:sample_start` keep their relative order.
    pub fn stable_sort_segments(&mut self) {
        self.draft
            .captures
            .sort_by_key(|capture| capture.sample_start);
        self.draft
            .annotations
            .sort_by_key(|annotation| annotation.sample_start);
        self.validation = None;
    }

    /// Validates the draft under `policy` and records the report for
    /// [`MetadataEdit::commit`].
    ///
    /// The returned report is stamped with the policy and a fingerprint of
    /// the draft, so it becomes stale if the draft changes afterwards. See
    /// [`Metadata::validate`] for the checks performed.
    pub fn validate(
        &mut self,
        policy: crate::validation::ValidationPolicy,
    ) -> crate::validation::ValidationReport {
        let revision = self.draft.revision_fingerprint();
        let mut report = self.draft.validate(policy);
        report.revision = Some(revision);
        report.policy = Some(policy);
        self.validation = Some(report.clone());
        report
    }

    /// Applies the draft to the original [`Metadata`], consuming the edit.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`], leaving the original
    /// document unchanged, if:
    ///
    /// * [`MetadataEdit::validate`] has not been called since the last
    ///   change;
    /// * the recorded report has status
    ///   [`Failed`](crate::validation::ValidationStatus::Failed); or
    /// * the draft no longer matches the fingerprint in the report.
    pub fn commit(self) -> Result<(), crate::SigmfError> {
        let report = self.validation.ok_or_else(|| {
            crate::SigmfError::InvalidMetadata(
                "metadata edits require a fresh validation report before commit".to_string(),
            )
        })?;
        if report.status == crate::validation::ValidationStatus::Failed {
            return Err(crate::SigmfError::InvalidMetadata(
                "metadata edit failed validation".to_string(),
            ));
        }
        let revision = self.draft.revision_fingerprint();
        if report.revision.as_deref() != Some(revision.as_str()) || report.policy.is_none() {
            return Err(crate::SigmfError::InvalidMetadata(
                "validation report is stale or belongs to a different context".to_string(),
            ));
        }
        *self.target = self.draft;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &[u8] = br#"{
        "global": {
            "core:version": "1.2.6",
            "core:datatype": "ri16_le",
            "vendor:global": {"enabled": true}
        },
        "captures": [{"core:sample_start": 0, "core:frequency": 915000000}],
        "annotations": [{"core:sample_start": 2, "vendor:label": "burst"}],
        "vendor:top": 9007199254740993
    }"#;

    #[test]
    fn metadata_round_trip_preserves_unknown_fields_and_omits_absent_options() {
        let metadata = Metadata::from_slice(METADATA).unwrap();
        let output = metadata.to_vec().unwrap();
        let value: Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(value["global"]["vendor:global"]["enabled"], true);
        assert_eq!(value["captures"][0]["core:frequency"], 915000000.0);
        assert_eq!(value["annotations"][0]["vendor:label"], "burst");
        assert_eq!(value["vendor:top"], 9007199254740993u64);
        assert!(value["global"].get("core:author").is_none());
        assert!(value["annotations"][0].get("core:sample_count").is_none());
        assert_eq!(metadata.original_bytes(), Some(METADATA));
    }

    #[test]
    fn original_bytes_are_invalidated_by_metadata_edits() {
        let mut metadata = Metadata::from_slice(METADATA).unwrap();
        assert!(metadata.original_digest().unwrap().is_some());
        metadata.set_description("edited".to_string());
        assert!(metadata.original_bytes().is_none());
        assert!(metadata.original_digest().unwrap().is_none());
    }

    #[test]
    fn required_fields_report_json_pointers() {
        let error = Metadata::from_slice(
            br#"{"global":{"core:datatype":"ri16_le"},"captures":[],"annotations":[]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("/global/core:version"), "{error}");

        let error = Metadata::from_slice(
            br#"{"global":{"core:version":"1.2.6","core:datatype":"ri16_le"}}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("/captures"), "{error}");
    }

    #[test]
    fn slice_parser_enforces_metadata_size_limit() {
        let oversized = vec![b' '; crate::MAX_META_SIZE + 1];
        assert!(matches!(
            Metadata::from_slice(&oversized),
            Err(crate::SigmfError::ResourceLimit(_))
        ));
    }

    #[test]
    fn known_keys_cannot_be_inserted_as_extensions() {
        let mut metadata = Metadata::from_slice(METADATA).unwrap();
        assert!(
            metadata
                .insert_global_extra("core:version", Value::String("2".into()))
                .is_err()
        );
        assert!(
            metadata
                .insert_extra("captures", Value::Array(Vec::new()))
                .is_err()
        );
        assert!(
            metadata
                .insert_extra("vendor:other", Value::Bool(true))
                .is_ok()
        );
    }

    #[test]
    fn queries_use_absolute_half_open_ranges_and_explicit_frequency_bounds() {
        let metadata = Metadata::from_slice(
            br#"{
                "global": {"core:version":"1.2.6","core:datatype":"ri16_le"},
                "captures":[{"core:sample_start":10},{"core:sample_start":20}],
                "annotations":[
                    {"core:sample_start":2,"core:sample_count":4,"core:freq_lower_edge":100,"core:freq_upper_edge":200},
                    {"core:sample_start":6,"core:sample_count":0,"core:freq_lower_edge":300}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(metadata.capture_at(19).unwrap().sample_start, 10);
        assert_eq!(metadata.capture_at(20).unwrap().sample_start, 20);
        assert!(metadata.capture_at(9).is_none());
        assert_eq!(
            metadata
                .annotations_intersecting_samples(5, 2)
                .unwrap()
                .len(),
            1
        );
        assert!(
            metadata
                .annotations_intersecting_frequency(200.0, 250.0)
                .unwrap()
                .len()
                == 1
        );
        assert!(
            metadata
                .annotations_intersecting_samples(u64::MAX, 1)
                .is_err()
        );
        assert!(
            metadata
                .annotations_intersecting_frequency(2.0, 1.0)
                .is_err()
        );
    }

    #[test]
    fn metadata_diff_uses_escaped_json_pointers() {
        let before = Metadata::from_slice(METADATA).unwrap();
        let mut after = before.clone();
        after
            .insert_extra("vendor/a~b", Value::String("changed".into()))
            .unwrap();
        let changes = before.diff(&after).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].json_pointer, "/vendor~1a~0b");
    }

    #[test]
    fn edit_commit_is_transactional_and_requires_current_validation() {
        let mut metadata = Metadata::from_slice(METADATA).unwrap();
        {
            let mut edit = metadata.edit();
            edit.set_description("reviewed".into());
            assert_eq!(
                edit.validate(crate::validation::ValidationPolicy::StrictSupportedVersion)
                    .status,
                crate::validation::ValidationStatus::NotChecked
            );
            edit.commit().unwrap();
        }
        assert_eq!(metadata.get_description().as_deref(), Some("reviewed"));

        let before = metadata.to_vec().unwrap();
        let mut edit = metadata.edit();
        edit.add_annotation(annotation::Annotation::new(
            4,
            None,
            Some(1.0),
            None,
            None,
            None,
            None,
            None,
        ));
        let report = edit.validate(crate::validation::ValidationPolicy::StrictSupportedVersion);
        assert_eq!(report.status, crate::validation::ValidationStatus::Failed);
        assert!(edit.commit().is_err());
        assert_eq!(metadata.to_vec().unwrap(), before);

        let mut edit = metadata.edit();
        let _ = edit.validate(crate::validation::ValidationPolicy::StrictSupportedVersion);
        edit.set_description("stale".into());
        assert!(edit.commit().is_err());
        assert_eq!(metadata.get_description().as_deref(), Some("reviewed"));
    }

    #[test]
    fn edit_stable_sort_normalizes_segments_transactionally() {
        let mut metadata = Metadata::from_slice(METADATA).unwrap();
        metadata.add_capture(capture::Capture::new(8));
        metadata.add_capture(capture::Capture::new(3));
        metadata.add_annotation(annotation::Annotation::new(
            9, None, None, None, None, None, None, None,
        ));
        metadata.add_annotation(annotation::Annotation::new(
            1, None, None, None, None, None, None, None,
        ));

        let mut edit = metadata.edit();
        edit.stable_sort_segments();
        assert_eq!(
            edit.validate(crate::validation::ValidationPolicy::StrictSupportedVersion)
                .status,
            crate::validation::ValidationStatus::NotChecked
        );
        edit.commit().unwrap();
        assert_eq!(
            metadata
                .captures()
                .iter()
                .map(|capture| capture.sample_start)
                .collect::<Vec<_>>(),
            [0, 3, 8]
        );
        assert_eq!(
            metadata
                .annotations()
                .iter()
                .map(|annotation| annotation.sample_start)
                .collect::<Vec<_>>(),
            [1, 2, 9]
        );
    }

    #[test]
    fn metadata_save_returns_digest_of_exact_bytes_and_does_not_overwrite() {
        let directory = std::env::temp_dir().join(format!(
            "sigmf-metadata-save-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("capture.sigmf-meta");
        let metadata = Metadata::from_slice(METADATA).unwrap();
        let digest = metadata
            .save_new(
                &path,
                crate::validation::ValidationPolicy::StrictSupportedVersion,
            )
            .unwrap();
        let written = std::fs::read(&path).unwrap();
        assert_eq!(digest, hash_from_slice(&written).unwrap());
        assert!(
            metadata
                .save_new(
                    &path,
                    crate::validation::ValidationPolicy::StrictSupportedVersion,
                )
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), written);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
