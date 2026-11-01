//! References from a SigMF collection to its member recordings.
//!
//! A SigMF Collection (`.sigmf-collection`, see [`crate::collection`]) lists
//! the recordings it groups in its `core:streams` array. Each entry is modeled
//! by [`RecordingRef`]: an object holding the recording's base `name` and the
//! SHA-512 `hash` of its metadata file. Unknown keys on an entry are preserved as extension fields.
//!
//! Older collections encode each entry as a two-element `[name, hash]` array;
//! such legacy tuples are accepted when a collection is parsed, and the
//! original form is remembered so that
//! [`Collection::to_vec_preserving_stream_forms`] can write it back unchanged.
//!
//! [`Collection::to_vec_preserving_stream_forms`]: crate::collection::Collection::to_vec_preserving_stream_forms

use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One entry of a SigMF collection's `core:streams` array.
///
/// Identifies a recording by its base `name` (the metadata file name without
/// the `.sigmf-meta` extension) and the SHA-512 `hash` of that metadata file,
/// as a lowercase hexadecimal string.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingRef {
    name: String,
    hash: String,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
    #[serde(skip)]
    legacy_tuple: bool,
}

impl RecordingRef {
    /// Creates a reference to the recording `name` whose metadata file has
    /// SHA-512 digest `hash`.
    ///
    /// Neither argument is validated. The reference has no extension fields
    /// and serializes as an object (not a legacy tuple).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::recording::RecordingRef;
    ///
    /// let reference = RecordingRef::new("example", "0123abcd");
    /// assert_eq!(reference.get_name(), "example");
    /// assert_eq!(reference.get_hash(), "0123abcd");
    ///
    /// let json = serde_json::to_value(&reference).unwrap();
    /// assert_eq!(json, serde_json::json!({"name": "example", "hash": "0123abcd"}));
    /// ```
    pub fn new(name: &str, hash: &str) -> Self {
        RecordingRef {
            name: name.to_string(),
            hash: hash.to_string(),
            extra_fields: Map::new(),
            legacy_tuple: false,
        }
    }

    /// Returns the recording's base name (the `name` key).
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the SHA-512 digest of the recording's metadata file (the
    /// `hash` key).
    pub fn get_hash(&self) -> &str {
        &self.hash
    }

    /// Returns the extension fields preserved on this entry, i.e. every key
    /// other than `name` and `hash`.
    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    /// Inserts or replaces an extension field on this entry.
    ///
    /// Returns the previous value stored under `key`, if any. Note that a
    /// reference parsed from a legacy `[name, hash]` tuple is written back as
    /// a tuple by [`Collection::to_vec_preserving_stream_forms`], which
    /// drops any extension fields added to it.
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`] if `key` is `name` or
    /// `hash`; use [`RecordingRef::set_name`] or [`RecordingRef::set_hash`]
    /// instead.
    ///
    /// [`Collection::to_vec_preserving_stream_forms`]: crate::collection::Collection::to_vec_preserving_stream_forms
    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, crate::SigmfError> {
        let key = key.into();
        if matches!(key.as_str(), "name" | "hash") {
            return Err(crate::SigmfError::InvalidMetadata(format!(
                "cannot insert known field {key:?} as an extension"
            )));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    /// Sets the recording's base name (the `name` key).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Sets the SHA-512 digest of the recording's metadata file (the `hash`
    /// key).
    pub fn set_hash(&mut self, hash: String) {
        self.hash = hash;
    }

    pub(crate) fn from_legacy_tuple(name: &str, hash: &str) -> Self {
        let mut reference = Self::new(name, hash);
        reference.legacy_tuple = true;
        reference
    }

    pub(crate) fn was_legacy_tuple(&self) -> bool {
        self.legacy_tuple
    }
}
