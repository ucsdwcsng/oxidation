use super::{Integer, Number};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Annotation {
    #[serde(rename = "core:sample_start")]
    pub sample_start: Integer,
    #[serde(rename = "core:sample_count", skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<Integer>,
    #[serde(
        rename = "core:freq_lower_edge",
        skip_serializing_if = "Option::is_none"
    )]
    pub freq_lower_edge: Option<Number>,
    #[serde(
        rename = "core:freq_upper_edge",
        skip_serializing_if = "Option::is_none"
    )]
    pub freq_upper_edge: Option<Number>,
    #[serde(rename = "core:label", skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "core:comment", skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "core:generator", skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(rename = "core:uuid", skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(flatten)]
    extra_fields: Map<String, Value>,
}

impl Annotation {
    pub fn builder(sample_start: Integer) -> AnnotationBuilder {
        AnnotationBuilder::new(sample_start)
    }

    #[allow(clippy::too_many_arguments)]
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

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub(crate) fn extension_fields_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extra_fields
    }

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
    pub fn set_sample_start(&mut self, sample_start: Integer) {
        self.sample_start = sample_start;
    }

    pub fn set_sample_count(&mut self, sample_count: Integer) {
        self.sample_count = Some(sample_count);
    }

    pub fn set_freq_lower_edge(&mut self, freq_lower_edge: Number) {
        self.freq_lower_edge = Some(freq_lower_edge);
    }

    pub fn set_freq_upper_edge(&mut self, freq_upper_edge: Number) {
        self.freq_upper_edge = Some(freq_upper_edge);
    }

    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    pub fn set_comment(&mut self, comment: String) {
        self.comment = Some(comment);
    }

    pub fn set_generator(&mut self, generator: String) {
        self.generator = Some(generator);
    }

    pub fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid);
    }

    // Getters
    pub fn get_sample_start(&self) -> Integer {
        self.sample_start
    }

    pub fn get_sample_count(&self) -> Option<Integer> {
        self.sample_count
    }

    pub fn get_freq_lower_edge(&self) -> Option<Number> {
        self.freq_lower_edge
    }

    pub fn get_freq_upper_edge(&self) -> Option<Number> {
        self.freq_upper_edge
    }

    pub fn get_label(&self) -> Option<String> {
        self.label.clone()
    }

    pub fn get_comment(&self) -> Option<String> {
        self.comment.clone()
    }

    pub fn get_generator(&self) -> Option<String> {
        self.generator.clone()
    }

    pub fn get_uuid(&self) -> Option<String> {
        self.uuid.clone()
    }
}

#[derive(Debug, Clone)]
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

    pub fn sample_count(mut self, sample_count: Integer) -> Self {
        self.sample_count = Some(sample_count);
        self
    }

    pub fn frequency_range(mut self, lower: Number, upper: Number) -> Self {
        self.freq_lower_edge = Some(lower);
        self.freq_upper_edge = Some(upper);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = Some(generator.into());
        self
    }

    pub fn uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

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
        assert!(Annotation::builder(0)
            .frequency_range(200.0, 100.0)
            .build()
            .is_err());
    }
}
