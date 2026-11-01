use super::{annotation, capture, global, utils::hash::hash_from_slice, Integer, Number};
use geojson::GeoJson;
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha512};
use std::io::Read;

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

    pub fn to_vec(&self) -> Result<Vec<u8>, crate::SigmfError> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn to_vec_pretty(&self) -> Result<Vec<u8>, crate::SigmfError> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

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

    pub fn save_new(
        &self,
        path: impl AsRef<std::path::Path>,
        policy: crate::validation::ValidationPolicy,
    ) -> Result<String, crate::SigmfError> {
        self.save_atomic(path.as_ref(), policy, false)
    }

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

    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, crate::SigmfError> {
        Self::from_reader(std::fs::File::open(path)?)
    }

    pub fn original_bytes(&self) -> Option<&[u8]> {
        self.original_bytes.as_deref()
    }

    pub fn original_digest(&self) -> Result<Option<String>, crate::SigmfError> {
        self.original_bytes
            .as_deref()
            .map(hash_from_slice)
            .transpose()
    }

    pub fn hash(&self) -> Result<String, crate::SigmfError> {
        let serialized = self.to_vec()?;
        hash_from_slice(&serialized)
    }

    fn revision_fingerprint(&self) -> String {
        let digest = Sha512::digest(format!("{self:?}").as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn global(&self) -> &global::Global {
        &self.global
    }

    pub fn captures(&self) -> &[capture::Capture] {
        &self.captures
    }

    pub fn annotations(&self) -> &[annotation::Annotation] {
        &self.annotations
    }

    pub fn extra_fields(&self) -> &Map<String, Value> {
        &self.extra_fields
    }

    pub fn capture_at(&self, sample_index: Integer) -> Option<&capture::Capture> {
        self.captures
            .iter()
            .filter(|capture| capture.sample_start <= sample_index)
            .max_by_key(|capture| capture.sample_start)
    }

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

    pub fn diff(&self, other: &Self) -> Result<Vec<MetadataChange>, crate::SigmfError> {
        let before = serde_json::to_value(self)?;
        let after = serde_json::to_value(other)?;
        let mut changes = Vec::new();
        collect_changes("", Some(&before), Some(&after), &mut changes);
        Ok(changes)
    }

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
    pub fn set_version(&mut self, version: String) {
        self.original_bytes = None;
        self.global.set_version(version);
    }

    pub fn set_datatype(&mut self, datatype: global::Datatype) {
        self.original_bytes = None;
        self.global.set_datatype(datatype);
    }

    pub fn set_sample_rate(&mut self, sample_rate: Number) {
        self.original_bytes = None;
        self.global.set_sample_rate(sample_rate);
    }

    pub fn set_author(&mut self, author: String) {
        self.original_bytes = None;
        self.global.set_author(author);
    }

    pub fn set_collection(&mut self, collection_str: String) {
        self.original_bytes = None;
        self.global.set_collection(collection_str);
    }

    pub fn set_data_doi(&mut self, data_doi: String) {
        self.original_bytes = None;
        self.global.set_data_doi(data_doi);
    }

    pub fn set_description(&mut self, description: String) {
        self.original_bytes = None;
        self.global.set_description(description);
    }

    pub fn set_hw(&mut self, hw: String) {
        self.original_bytes = None;
        self.global.set_hw(hw);
    }

    pub fn set_license(&mut self, license: String) {
        self.original_bytes = None;
        self.global.set_license(license);
    }

    pub fn set_metadata_only(&mut self, metadata_only: bool) {
        self.original_bytes = None;
        self.global.set_metadata_only(metadata_only);
    }

    pub fn set_meta_doi(&mut self, meta_doi: String) {
        self.original_bytes = None;
        self.global.set_meta_doi(meta_doi);
    }

    pub fn set_num_channels(&mut self, num_channels: Integer) {
        self.original_bytes = None;
        self.global.set_num_channels(num_channels);
    }

    pub fn set_offset(&mut self, offset: Integer) {
        self.original_bytes = None;
        self.global.set_offset(offset);
    }

    pub fn set_recorder(&mut self, recorder: String) {
        self.original_bytes = None;
        self.global.set_recorder(recorder);
    }

    pub fn set_sha512(&mut self, hash: String) {
        self.original_bytes = None;
        self.global.set_sha512(hash);
    }

    pub fn set_geolocation(&mut self, geolocation: GeoJson) {
        self.original_bytes = None;
        self.global.set_geolocation(geolocation);
    }

    pub fn set_extensions(&mut self, extensions: Vec<global::ExtensionDeclaration>) {
        self.original_bytes = None;
        self.global.set_extensions(extensions);
    }

    pub fn set_dataset(&mut self, dataset: String) {
        self.original_bytes = None;
        self.global.set_dataset(dataset);
    }

    pub fn set_trailing_bytes(&mut self, trailing_bytes: Integer) {
        self.original_bytes = None;
        self.global.set_trailing_bytes(trailing_bytes);
    }

    // ---------------- Annotations ---------------- //
    pub fn add_annotation(&mut self, annotation: annotation::Annotation) {
        self.original_bytes = None;
        self.annotations.push(annotation);
    }

    // ----------------- Captures ------------------ //
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
    pub fn get_version(&self) -> String {
        self.global.get_version()
    }

    pub fn get_datatype(&self) -> global::Datatype {
        self.global.get_datatype()
    }

    pub fn get_sample_rate(&self) -> Option<Number> {
        self.global.get_sample_rate()
    }

    pub fn get_author(&self) -> Option<String> {
        self.global.get_author()
    }

    pub fn get_collection(&self) -> Option<String> {
        self.global.get_collection()
    }

    pub fn get_data_doi(&self) -> Option<String> {
        self.global.get_data_doi()
    }

    pub fn get_description(&self) -> Option<String> {
        self.global.get_description()
    }

    pub fn get_hw(&self) -> Option<String> {
        self.global.get_hw()
    }

    pub fn get_license(&self) -> Option<String> {
        self.global.get_license()
    }

    pub fn get_metadata_only(&self) -> Option<bool> {
        self.global.get_metadata_only()
    }

    pub fn get_meta_doi(&self) -> Option<String> {
        self.global.get_meta_doi()
    }

    pub fn get_num_channels(&self) -> Option<Integer> {
        self.global.get_num_channels()
    }

    pub fn get_offset(&self) -> Option<Integer> {
        self.global.get_offset()
    }

    pub fn get_recorder(&self) -> Option<String> {
        self.global.get_recorder()
    }

    pub fn get_sha512(&self) -> Option<String> {
        self.global.get_sha512()
    }

    pub fn get_geolocation(&self) -> Option<GeoJson> {
        self.global.get_geolocation()
    }

    pub fn get_extensions(&self) -> Option<Vec<global::ExtensionDeclaration>> {
        self.global.get_extensions()
    }

    pub fn get_dataset(&self) -> Option<String> {
        self.global.get_dataset()
    }

    pub fn get_trailing_bytes(&self) -> Option<Integer> {
        self.global.get_trailing_bytes()
    }

    // ---------------- Annotations ---------------- //
    pub fn get_annotations(&self) -> Vec<annotation::Annotation> {
        self.annotations.clone()
    }

    // ----------------- Captures ------------------ //
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

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataChange {
    pub json_pointer: String,
    pub before: Option<Value>,
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

pub struct MetadataEdit<'a> {
    target: &'a mut Metadata,
    draft: Metadata,
    validation: Option<crate::validation::ValidationReport>,
}

impl MetadataEdit<'_> {
    pub fn metadata(&self) -> &Metadata {
        &self.draft
    }

    pub fn metadata_mut(&mut self) -> &mut Metadata {
        self.validation = None;
        self.draft.original_bytes = None;
        &mut self.draft
    }

    pub fn set_description(&mut self, description: String) {
        self.draft.set_description(description);
        self.validation = None;
    }

    pub fn set_datatype(&mut self, datatype: global::Datatype) {
        self.draft.set_datatype(datatype);
        self.validation = None;
    }

    pub fn set_offset(&mut self, offset: Integer) {
        self.draft.set_offset(offset);
        self.validation = None;
    }

    pub fn add_annotation(&mut self, annotation: annotation::Annotation) {
        self.draft.add_annotation(annotation);
        self.validation = None;
    }

    pub fn replace_annotation(
        &mut self,
        index: usize,
        annotation: annotation::Annotation,
    ) -> Option<annotation::Annotation> {
        let current = self.draft.annotations.get_mut(index)?;
        self.validation = None;
        Some(std::mem::replace(current, annotation))
    }

    pub fn add_capture(&mut self, capture: capture::Capture) {
        self.draft.add_capture(capture);
        self.validation = None;
    }

    pub fn replace_capture(
        &mut self,
        index: usize,
        capture: capture::Capture,
    ) -> Option<capture::Capture> {
        let current = self.draft.captures.get_mut(index)?;
        self.validation = None;
        Some(std::mem::replace(current, capture))
    }

    pub fn remove_annotation(&mut self, index: usize) -> Option<annotation::Annotation> {
        if index < self.draft.annotations.len() {
            self.validation = None;
            Some(self.draft.annotations.remove(index))
        } else {
            None
        }
    }

    pub fn remove_capture(&mut self, index: usize) -> Option<capture::Capture> {
        if index < self.draft.captures.len() {
            self.validation = None;
            Some(self.draft.captures.remove(index))
        } else {
            None
        }
    }

    pub fn stable_sort_segments(&mut self) {
        self.draft
            .captures
            .sort_by_key(|capture| capture.sample_start);
        self.draft
            .annotations
            .sort_by_key(|annotation| annotation.sample_start);
        self.validation = None;
    }

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
        assert!(metadata
            .insert_global_extra("core:version", Value::String("2".into()))
            .is_err());
        assert!(metadata
            .insert_extra("captures", Value::Array(Vec::new()))
            .is_err());
        assert!(metadata
            .insert_extra("vendor:other", Value::Bool(true))
            .is_ok());
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
        assert!(metadata
            .annotations_intersecting_samples(u64::MAX, 1)
            .is_err());
        assert!(metadata
            .annotations_intersecting_frequency(2.0, 1.0)
            .is_err());
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
        assert!(metadata
            .save_new(
                &path,
                crate::validation::ValidationPolicy::StrictSupportedVersion,
            )
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), written);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
