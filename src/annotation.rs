//! SigMF annotation segments.
//!
//! A SigMF metadata file has an `annotations` array whose entries describe
//! features of interest in the recording, such as a detected burst, a
//! labeled emitter, or a free-form comment. Each entry covers a range of
//! samples starting at `core:sample_start` and may optionally be bounded in
//! frequency. This module models a single entry as [`Annotation`] and
//! provides [`AnnotationBuilder`] for constructing one with validated
//! frequency bounds.
//!
//! Keys in the `core` namespace map to typed fields on [`Annotation`]; any
//! other keys (for example, extension fields) are preserved verbatim and
//! exposed through [`Annotation::extra_fields`].

use super::{Integer, Number};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A single entry of the SigMF `annotations` array.
///
/// Optional `core` fields are stored as [`Option`]s and are omitted from
/// serialized JSON when `None`. Unknown keys encountered during
/// deserialization (typically extension fields) are retained and serialized
/// back out unchanged; see [`Annotation::extra_fields`] and
/// [`Annotation::insert_extra`].
///
/// Prefer [`Annotation::builder`], which checks frequency bounds, over
/// [`Annotation::new`], which performs no validation.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Annotation {
    /// Index of the first sample covered by this annotation (`core:sample_start`).
    #[serde(rename = "core:sample_start")]
    pub sample_start: Integer,
    /// Number of samples covered by this annotation (`core:sample_count`).
    ///
    /// Per the SigMF specification, an absent value means the annotation
    /// extends to the end of the recording.
    #[serde(rename = "core:sample_count", skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<Integer>,
    /// Lower edge of the annotated feature's frequency range in Hz
    /// (`core:freq_lower_edge`).
    #[serde(
        rename = "core:freq_lower_edge",
        skip_serializing_if = "Option::is_none"
    )]
    pub freq_lower_edge: Option<Number>,
    /// Upper edge of the annotated feature's frequency range in Hz
    /// (`core:freq_upper_edge`).
    #[serde(
        rename = "core:freq_upper_edge",
        skip_serializing_if = "Option::is_none"
    )]
    pub freq_upper_edge: Option<Number>,
    /// Short human-readable label for the annotation (`core:label`).
    #[serde(rename = "core:label", skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Free-text human-readable comment (`core:comment`).
    #[serde(rename = "core:comment", skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Name of the entity (person or software) that created the annotation
    /// (`core:generator`).
    #[serde(rename = "core:generator", skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    /// RFC 4122 UUID identifying this annotation (`core:uuid`).
    #[serde(rename = "core:uuid", skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Annotation {
    /// Starts building an [`Annotation`] that begins at `sample_start`
    /// (`core:sample_start`).
    ///
    /// Equivalent to [`AnnotationBuilder::new`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::annotation::Annotation;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let annotation = Annotation::builder(4096)
    ///     .sample_count(1024)
    ///     .frequency_range(2.400e9, 2.402e9)
    ///     .label("burst")
    ///     .build()?;
    /// assert_eq!(annotation.get_sample_start(), 4096);
    /// assert_eq!(annotation.get_sample_count(), Some(1024));
    ///
    /// let json = serde_json::to_value(&annotation)?;
    /// assert_eq!(json["core:label"], "burst");
    /// assert!(json.get("core:comment").is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder(sample_start: Integer) -> AnnotationBuilder {
        AnnotationBuilder::new(sample_start)
    }

    #[allow(clippy::too_many_arguments)]
    /// Creates an [`Annotation`] directly from all of its `core` fields, with no
    /// extra fields.
    ///
    /// Unlike [`AnnotationBuilder::build`], this performs no validation; in
    /// particular, frequency bounds are not checked for finiteness, ordering, or
    /// being supplied together.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sigmf::annotation::Annotation;
    ///
    /// let annotation = Annotation::new(
    ///     0,
    ///     Some(1000),
    ///     None,
    ///     None,
    ///     Some("preamble".to_string()),
    ///     None,
    ///     Some("my-detector".to_string()),
    ///     None,
    /// );
    /// assert_eq!(annotation.get_label().as_deref(), Some("preamble"));
    /// assert!(annotation.extra_fields().is_empty());
    /// ```
    pub fn new(
        sample_start: Integer,
        sample_count: Option<Integer>,
        freq_lower_edge: Option<Number>,
        freq_upper_edge: Option<Number>,
        label: Option<String>,
        comment: Option<String>,
        generator: Option<String>,
        uuid: Option<String>,
    ) -> Self {
        Annotation {
            sample_start,
            sample_count,
            freq_lower_edge,
            freq_upper_edge,
            label,
            comment,
            generator,
            uuid,
            extra_fields: Map::new(),
        }
    }

    /// Returns the fields of this annotation that are not modeled by a typed
    /// field, keyed by their full JSON key.
    ///
    /// These are the keys that did not map to a typed field during
    /// deserialization (typically extension fields), plus any added via
    /// [`Annotation::insert_extra`]. They are serialized alongside the typed
    /// `core` fields.
    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn extension_fields_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extra_fields
    }

    /// Inserts or replaces an extra (non-typed) field, returning the previous
    /// value for `key` if one was present.
    ///
    /// The key is stored as given; namespace syntax and extension declarations
    /// are not checked here (see [`crate::validation`]).
    ///
    /// # Errors
    ///
    /// Returns an error message if `key` is one of the `core:` annotation keys
    /// modeled by a typed field on [`Annotation`] (for example
    /// `core:sample_start`); set those through the corresponding field or setter
    /// instead.
    pub fn insert_extra(
        &mut self,
        key: impl Into<String>,
        value: Value,
    ) -> Result<Option<Value>, String> {
        let key = key.into();
        if matches!(
            key.as_str(),
            "core:sample_start"
                | "core:sample_count"
                | "core:freq_lower_edge"
                | "core:freq_upper_edge"
                | "core:label"
                | "core:comment"
                | "core:generator"
                | "core:uuid"
        ) {
            return Err(format!("cannot insert known field {key:?} as an extension"));
        }
        Ok(self.extra_fields.insert(key, value))
    }

    // Setters
    /// Sets the index of the first annotated sample (`core:sample_start`).
    pub fn set_sample_start(&mut self, sample_start: Integer) {
        self.sample_start = sample_start;
    }

    /// Sets the number of annotated samples (`core:sample_count`).
    pub fn set_sample_count(&mut self, sample_count: Integer) {
        self.sample_count = Some(sample_count);
    }

    /// Sets the lower frequency edge in Hz (`core:freq_lower_edge`).
    ///
    /// The value is not validated against the upper edge.
    pub fn set_freq_lower_edge(&mut self, freq_lower_edge: Number) {
        self.freq_lower_edge = Some(freq_lower_edge);
    }

    /// Sets the upper frequency edge in Hz (`core:freq_upper_edge`).
    ///
    /// The value is not validated against the lower edge.
    pub fn set_freq_upper_edge(&mut self, freq_upper_edge: Number) {
        self.freq_upper_edge = Some(freq_upper_edge);
    }

    /// Sets the short human-readable label (`core:label`).
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    /// Sets the free-text comment (`core:comment`).
    pub fn set_comment(&mut self, comment: String) {
        self.comment = Some(comment);
    }

    /// Sets the name of the annotation's creator (`core:generator`).
    pub fn set_generator(&mut self, generator: String) {
        self.generator = Some(generator);
    }

    /// Sets the annotation UUID (`core:uuid`).
    ///
    /// The value is stored as given; it is not checked to be a valid UUID.
    pub fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid);
    }

    // Getters
    /// Returns the index of the first annotated sample (`core:sample_start`).
    pub fn get_sample_start(&self) -> Integer {
        self.sample_start
    }

    /// Returns the number of annotated samples (`core:sample_count`), if set.
    pub fn get_sample_count(&self) -> Option<Integer> {
        self.sample_count
    }

    /// Returns the lower frequency edge in Hz (`core:freq_lower_edge`), if set.
    pub fn get_freq_lower_edge(&self) -> Option<Number> {
        self.freq_lower_edge
    }

    /// Returns the upper frequency edge in Hz (`core:freq_upper_edge`), if set.
    pub fn get_freq_upper_edge(&self) -> Option<Number> {
        self.freq_upper_edge
    }

    /// Returns a copy of the label (`core:label`), if set.
    pub fn get_label(&self) -> Option<String> {
        self.label.clone()
    }

    /// Returns a copy of the comment (`core:comment`), if set.
    pub fn get_comment(&self) -> Option<String> {
        self.comment.clone()
    }

    /// Returns a copy of the creator name (`core:generator`), if set.
    pub fn get_generator(&self) -> Option<String> {
        self.generator.clone()
    }

    /// Returns a copy of the annotation UUID (`core:uuid`), if set.
    pub fn get_uuid(&self) -> Option<String> {
        self.uuid.clone()
    }
}

#[derive(Debug, Clone)]
/// Builder for [`Annotation`] that validates frequency bounds on
/// [`build`](AnnotationBuilder::build).
///
/// Create one with [`Annotation::builder`] or [`AnnotationBuilder::new`].
/// All fields other than `core:sample_start` start unset. The built
/// annotation has no extra fields.
pub struct AnnotationBuilder {
    sample_start: Integer,
    sample_count: Option<Integer>,
    freq_lower_edge: Option<Number>,
    freq_upper_edge: Option<Number>,
    label: Option<String>,
    comment: Option<String>,
    generator: Option<String>,
    uuid: Option<String>,
}

impl AnnotationBuilder {
    /// Creates a builder for an annotation beginning at `sample_start`
    /// (`core:sample_start`), with all other fields unset.
    pub fn new(sample_start: Integer) -> Self {
        Self {
            sample_start,
            sample_count: None,
            freq_lower_edge: None,
            freq_upper_edge: None,
            label: None,
            comment: None,
            generator: None,
            uuid: None,
        }
    }

    /// Sets the number of annotated samples (`core:sample_count`).
    ///
    /// Zero is accepted.
    pub fn sample_count(mut self, sample_count: Integer) -> Self {
        self.sample_count = Some(sample_count);
        self
    }

    /// Sets the lower and upper frequency edges in Hz
    /// (`core:freq_lower_edge` and `core:freq_upper_edge`).
    ///
    /// The bounds are validated by [`build`](AnnotationBuilder::build).
    pub fn frequency_range(mut self, lower: Number, upper: Number) -> Self {
        self.freq_lower_edge = Some(lower);
        self.freq_upper_edge = Some(upper);
        self
    }

    /// Sets the short human-readable label (`core:label`).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the free-text comment (`core:comment`).
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Sets the name of the annotation's creator (`core:generator`).
    pub fn generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = Some(generator.into());
        self
    }

    /// Sets the annotation UUID (`core:uuid`).
    ///
    /// The value is not checked to be a valid UUID.
    pub fn uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

    /// Validates the frequency bounds and builds the [`Annotation`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::SigmfError::InvalidMetadata`] if frequency bounds were
    /// supplied and either bound is not finite, or the lower bound exceeds the
    /// upper bound.
    pub fn build(self) -> Result<Annotation, crate::SigmfError> {
        match (self.freq_lower_edge, self.freq_upper_edge) {
            (Some(lower), Some(upper))
                if lower.is_finite() && upper.is_finite() && lower <= upper => {}
            (None, None) => {}
            (Some(lower), Some(upper)) if !lower.is_finite() || !upper.is_finite() => {
                return Err(crate::SigmfError::InvalidMetadata(
                    "annotation frequency bounds must be finite".into(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(crate::SigmfError::InvalidMetadata(
                    "annotation lower frequency bound exceeds its upper bound".into(),
                ));
            }
            _ => {
                return Err(crate::SigmfError::InvalidMetadata(
                    "annotation frequency bounds must be supplied together".into(),
                ));
            }
        }
        Ok(Annotation {
            sample_start: self.sample_start,
            sample_count: self.sample_count,
            freq_lower_edge: self.freq_lower_edge,
            freq_upper_edge: self.freq_upper_edge,
            label: self.label,
            comment: self.comment,
            generator: self.generator,
            uuid: self.uuid,
            extra_fields: Map::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_checks_frequency_bounds_and_accepts_zero_sample_count() {
        let annotation = Annotation::builder(4096)
            .sample_count(0)
            .frequency_range(100.0, 200.0)
            .label("burst")
            .build()
            .unwrap();
        assert_eq!(annotation.sample_start, 4096);
        assert_eq!(annotation.sample_count, Some(0));
        assert_eq!(annotation.label.as_deref(), Some("burst"));
        assert!(
            Annotation::builder(0)
                .frequency_range(200.0, 100.0)
                .build()
                .is_err()
        );
    }
}
