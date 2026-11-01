//! Semantic validation of SigMF metadata and collections.
//!
//! Validation complements parsing: a document that deserializes successfully
//! may still violate SigMF rules or use features this crate cannot vouch for.
//! The entry points are [`Metadata::validate`], [`Collection::validate`], their
//! `validate_with_registry` variants, and
//! [`Archive::validate_collection_references`](crate::Archive::validate_collection_references).
//! Each returns a [`ValidationReport`] listing every [`ValidationIssue`] found
//! and an overall [`ValidationStatus`].
//!
//! # What is checked
//!
//! For `.sigmf-meta` documents ([`Metadata`]):
//!
//! - `core:version` is the certified version (`1.2.6`);
//! - `core:sample_rate` is finite, positive, and at most 10<sup>12</sup>;
//! - indices and counts (`core:num_channels`, `core:offset`,
//!   `core:sample_start`, `core:sample_count`, ...) fit in 2<sup>63</sup> − 1,
//!   and `core:num_channels` is non-zero;
//! - `core:sha512` is 128 hexadecimal characters;
//! - frequencies are finite, annotation frequency edges are paired and ordered,
//!   and annotation sample ranges do not overflow;
//! - `core:datetime` uses the UTC `YYYY-MM-DDTHH:MM:SS[.fraction]Z` form;
//! - `core:geolocation` is a GeoJSON `Point` with two or three finite coordinates;
//! - captures and annotations are sorted by `core:sample_start`, and sample
//!   indices are not before `core:offset`;
//! - unrecognized `core:` fields, and extension fields whose namespace is not
//!   declared in `core:extensions`;
//! - declared extensions are checked against an optional
//!   [`ExtensionRegistry`](crate::extensions::ExtensionRegistry) for support,
//!   scope, and field names.
//!
//! For `.sigmf-collection` documents ([`Collection`]): version, recording
//! names (relative, no `..`, no backslashes, no empty segments), duplicate
//! references, metadata digests, unknown `core:` fields, and extensions.
//!
//! # Policies
//!
//! A [`ValidationPolicy`] controls how strictly portability concerns are
//! treated. Under [`ValidationPolicy::Interoperability`] an unsupported version,
//! unknown `core:` fields, and unregistered required extensions are warnings;
//! under [`ValidationPolicy::StrictSupportedVersion`] they are errors.
//!
//! # Example
//!
//! ```rust
//! use sigmf::datatype::Datatype;
//! use sigmf::global::Global;
//! use sigmf::meta::Metadata;
//! use sigmf::validation::{ValidationPolicy, ValidationStatus};
//! use std::str::FromStr;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut metadata = Metadata::new(
//!     Global::new("1.2.6", Datatype::from_str("cf32_le")?),
//!     Vec::new(),
//!     Vec::new(),
//! );
//! metadata.set_sample_rate(1e6);
//! let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
//! assert_eq!(report.status, ValidationStatus::Passed);
//!
//! metadata.set_sample_rate(-1.0);
//! let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
//! assert_eq!(report.status, ValidationStatus::Failed);
//! assert!(report
//!     .issues
//!     .iter()
//!     .any(|issue| issue.code == "sample-rate-out-of-range"));
//! # Ok(())
//! # }
//! ```

use crate::collection::Collection;
use crate::meta::Metadata;
use geojson::{GeoJson, GeometryValue};
use std::collections::HashSet;

/// How strictly validation treats content this implementation cannot fully verify.
///
/// The policy only affects the severity of portability-related issues
/// (unsupported `core:version`, unknown `core:` fields, and unregistered
/// required extensions). Structural and value errors are always
/// [`Severity::Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPolicy {
    /// Accept documents that may be valid under other SigMF versions or with
    /// extensions this application does not understand.
    ///
    /// Unsupported versions, unknown `core:` fields, and unregistered required
    /// extensions are reported as [`Severity::Warning`].
    Interoperability,
    /// Require a version and feature set this implementation certifies.
    ///
    /// Unsupported versions, unknown `core:` fields, and unregistered required
    /// extensions are reported as [`Severity::Error`].
    StrictSupportedVersion,
}

/// Severity of a [`ValidationIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A violation of the specification or of the selected policy; causes
    /// [`ValidationStatus::Failed`].
    Error,
    /// A recommendation violation or a condition that could not be verified.
    Warning,
    /// Purely informational; does not affect the outcome.
    Information,
}

/// Overall outcome of a validation run.
///
/// The status is derived from the report's issues in priority order:
///
/// 1. [`Failed`](Self::Failed) if any issue has [`Severity::Error`];
/// 2. otherwise [`NotChecked`](Self::NotChecked) if any issue indicates content
///    that could not be verified (issue codes `unsupported-version`,
///    `unknown-core-field`, `required-extension-unverified`,
///    `optional-extension-unverified`, `undeclared-extension-field`,
///    `unrecognized-extension-field`, or `unresolved-recording-reference`);
/// 3. otherwise [`Passed`](Self::Passed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// No errors and nothing left unverified.
    ///
    /// Other warnings (for example `sample-index-before-offset`) may still be
    /// present in [`ValidationReport::issues`].
    Passed,
    /// At least one issue has [`Severity::Error`].
    Failed,
    /// No errors were found, but some content could not be verified, such as
    /// an unsupported version, an unknown `core:` field, or an extension
    /// without registered support.
    NotChecked,
}

/// A single problem found during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Stable, machine-readable identifier such as `"segments-not-sorted"`.
    pub code: &'static str,
    /// Severity of the issue under the policy used for validation.
    pub severity: Severity,
    /// [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) JSON Pointer to the
    /// offending location, e.g. `"/captures/0/core:datetime"`.
    pub json_pointer: String,
    /// The `core:version` declared by the validated document.
    pub declared_version: String,
    /// Human-readable description of the problem.
    pub message: String,
    /// Optional suggestion for resolving the problem.
    pub remediation: Option<String>,
}

/// The result of validating a SigMF document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// Overall outcome derived from [`issues`](Self::issues); see
    /// [`ValidationStatus`].
    pub status: ValidationStatus,
    /// Every issue found, in the order checks were performed.
    pub issues: Vec<ValidationIssue>,
    /// Optional revision identifier for the validated document; validation
    /// itself leaves this as `None`.
    pub revision: Option<String>,
    /// The policy used to produce the report, if recorded.
    ///
    /// Set by [`Metadata::validate`] and [`Metadata::validate_with_registry`];
    /// `None` for collection and archive-reference reports.
    pub policy: Option<ValidationPolicy>,
}

impl ValidationReport {
    pub(crate) fn new(issues: Vec<ValidationIssue>) -> Self {
        let status = if issues.iter().any(|issue| issue.severity == Severity::Error) {
            ValidationStatus::Failed
        } else if issues.iter().any(|issue| {
            matches!(
                issue.code,
                "unsupported-version"
                    | "unknown-core-field"
                    | "required-extension-unverified"
                    | "optional-extension-unverified"
                    | "undeclared-extension-field"
                    | "unrecognized-extension-field"
                    | "unresolved-recording-reference"
            )
        }) {
            ValidationStatus::NotChecked
        } else {
            ValidationStatus::Passed
        };
        Self {
            status,
            issues,
            revision: None,
            policy: None,
        }
    }

    /// Returns `true` only if [`status`](Self::status) is
    /// [`ValidationStatus::Passed`].
    ///
    /// Reports with status [`ValidationStatus::NotChecked`] are not
    /// considered valid.
    pub fn is_valid(&self) -> bool {
        self.status == ValidationStatus::Passed
    }
}

impl Metadata {
    /// Validates this metadata against SigMF rules using `policy`.
    ///
    /// Extensions declared in `core:extensions` are reported as unverified
    /// because no [`ExtensionRegistry`](crate::extensions::ExtensionRegistry)
    /// is supplied; use [`validate_with_registry`](Self::validate_with_registry)
    /// to verify them. See the [module documentation](self) for the full list
    /// of checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sigmf::datatype::Datatype;
    /// use sigmf::global::Global;
    /// use sigmf::meta::Metadata;
    /// use sigmf::validation::{Severity, ValidationPolicy, ValidationStatus};
    /// use std::str::FromStr;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // An older version is tolerated under the interoperability policy...
    /// let metadata = Metadata::new(
    ///     Global::new("1.0.0", Datatype::from_str("ri16_le")?),
    ///     Vec::new(),
    ///     Vec::new(),
    /// );
    /// let report = metadata.validate(ValidationPolicy::Interoperability);
    /// assert_eq!(report.status, ValidationStatus::NotChecked);
    /// assert_eq!(report.issues[0].severity, Severity::Warning);
    ///
    /// // ...but rejected under the strict policy.
    /// let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
    /// assert_eq!(report.status, ValidationStatus::Failed);
    /// assert!(!report.is_valid());
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self, policy: ValidationPolicy) -> ValidationReport {
        self.validate_with_optional_registry(policy, None)
    }

    /// Validates this metadata, using `registry` to verify declared extensions.
    ///
    /// In addition to the checks performed by [`validate`](Self::validate),
    /// extensions declared in `core:extensions` whose name and version are
    /// registered are treated as verified, and their fields in the global,
    /// capture, and annotation objects are checked for the correct
    /// [`ExtensionScope`](crate::extensions::ExtensionScope) (an error) and for
    /// being declared by the extension (a warning).
    pub fn validate_with_registry(
        &self,
        policy: ValidationPolicy,
        registry: &crate::extensions::ExtensionRegistry,
    ) -> ValidationReport {
        self.validate_with_optional_registry(policy, Some(registry))
    }

    fn validate_with_optional_registry(
        &self,
        policy: ValidationPolicy,
        registry: Option<&crate::extensions::ExtensionRegistry>,
    ) -> ValidationReport {
        let mut issues = Vec::new();
        let version = self.global().version.as_str();
        if version != "1.2.6" {
            let severity = match policy {
                ValidationPolicy::Interoperability => Severity::Warning,
                ValidationPolicy::StrictSupportedVersion => Severity::Error,
            };
            push_issue(
                &mut issues,
                "unsupported-version",
                severity,
                "/global/core:version",
                version,
                format!("SigMF {version} is not certified by this implementation"),
                Some(
                    "Use a supported version or validate with an implementation that certifies it."
                        .into(),
                ),
            );
        }

        let global = self.global();
        if global.sample_rate.is_some_and(|number| !number.is_finite()) {
            push_issue(
                &mut issues,
                "non-finite-number",
                Severity::Error,
                "/global/core:sample_rate",
                version,
                "metadata numbers must be finite JSON numbers".into(),
                None,
            );
        }
        if let Some(sample_rate) = global.sample_rate
            && (sample_rate <= 0.0 || sample_rate > 1_000_000_000_000.0)
        {
            push_issue(
                &mut issues,
                "sample-rate-out-of-range",
                Severity::Error,
                "/global/core:sample_rate",
                version,
                "sample rate must be greater than zero and at most 1,000,000,000,000".into(),
                None,
            );
        }
        check_geolocation(
            &mut issues,
            global.geolocation.as_ref(),
            "/global/core:geolocation",
            version,
        );
        check_index(
            &mut issues,
            global.num_channels,
            "/global/core:num_channels",
            version,
        );
        check_index(&mut issues, global.offset, "/global/core:offset", version);
        check_index(
            &mut issues,
            global.trailing_bytes,
            "/global/core:trailing_bytes",
            version,
        );
        if global.num_channels == Some(0) {
            push_issue(
                &mut issues,
                "zero-channel-count",
                Severity::Error,
                "/global/core:num_channels",
                version,
                "channel count must be greater than zero".into(),
                None,
            );
        }
        if let Some(hash) = &global.sha512
            && (hash.len() != 128 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            push_issue(
                &mut issues,
                "invalid-dataset-digest",
                Severity::Error,
                "/global/core:sha512",
                version,
                "dataset digest must contain exactly 128 hexadecimal characters".into(),
                Some("Recompute the digest from the dataset-file bytes.".into()),
            );
        }

        check_extra_core_fields(
            &mut issues,
            global.extra_fields(),
            "/global",
            version,
            policy,
        );
        let declarations = global.extensions.as_deref().unwrap_or(&[]);
        check_extension_fields(
            &mut issues,
            global.extra_fields(),
            crate::extensions::ExtensionScope::Global,
            "/global",
            declarations,
            registry,
            version,
        );
        check_extra_core_fields(&mut issues, self.extra_fields(), "", version, policy);

        for (index, capture) in self.captures().iter().enumerate() {
            let pointer = format!("/captures/{index}");
            check_offset_bound(
                &mut issues,
                capture.sample_start,
                global.offset.unwrap_or(0),
                &format!("{pointer}/core:sample_start"),
                version,
            );
            check_index(
                &mut issues,
                Some(capture.sample_start),
                &format!("{pointer}/core:sample_start"),
                version,
            );
            check_finite(
                &mut issues,
                capture.frequency,
                &format!("{pointer}/core:frequency"),
                version,
            );
            if let Some(datetime) = &capture.datetime
                && !is_sigmf_timestamp(datetime)
            {
                push_issue(
                    &mut issues,
                    "invalid-utc-timestamp",
                    Severity::Error,
                    &format!("{pointer}/core:datetime"),
                    version,
                    "timestamp must use UTC YYYY-MM-DDTHH:MM:SS[.fraction]Z form".into(),
                    None,
                );
            }
            check_geolocation(
                &mut issues,
                capture.geolocation.as_ref(),
                &format!("{pointer}/core:geolocation"),
                version,
            );
            check_index(
                &mut issues,
                capture.global_index,
                &format!("{pointer}/core:global_index"),
                version,
            );
            check_index(
                &mut issues,
                capture.header_bytes,
                &format!("{pointer}/core:header_bytes"),
                version,
            );
            check_extra_core_fields(
                &mut issues,
                capture.extra_fields(),
                &pointer,
                version,
                policy,
            );
            check_extension_fields(
                &mut issues,
                capture.extra_fields(),
                crate::extensions::ExtensionScope::Capture,
                &pointer,
                declarations,
                registry,
                version,
            );
        }
        check_sorted(
            &mut issues,
            self.captures().iter().map(|capture| capture.sample_start),
            "/captures",
            version,
        );

        for (index, annotation) in self.annotations().iter().enumerate() {
            let pointer = format!("/annotations/{index}");
            check_offset_bound(
                &mut issues,
                annotation.sample_start,
                global.offset.unwrap_or(0),
                &format!("{pointer}/core:sample_start"),
                version,
            );
            check_index(
                &mut issues,
                Some(annotation.sample_start),
                &format!("{pointer}/core:sample_start"),
                version,
            );
            check_index(
                &mut issues,
                annotation.sample_count,
                &format!("{pointer}/core:sample_count"),
                version,
            );
            if let Some(sample_count) = annotation.sample_count {
                match annotation.sample_start.checked_add(sample_count) {
                    None => push_issue(
                        &mut issues,
                        "sample-range-overflow",
                        Severity::Error,
                        &pointer,
                        version,
                        "annotation sample range overflows the supported index type".into(),
                        None,
                    ),
                    Some(end) if end > i64::MAX as u64 => push_issue(
                        &mut issues,
                        "sample-range-out-of-range",
                        Severity::Error,
                        &pointer,
                        version,
                        "annotation sample range exceeds the SigMF 1.2.6 index limit".into(),
                        None,
                    ),
                    Some(_) => {}
                }
            }
            check_finite(
                &mut issues,
                annotation.freq_lower_edge,
                &format!("{pointer}/core:freq_lower_edge"),
                version,
            );
            check_finite(
                &mut issues,
                annotation.freq_upper_edge,
                &format!("{pointer}/core:freq_upper_edge"),
                version,
            );
            if annotation.freq_lower_edge.is_some() != annotation.freq_upper_edge.is_some() {
                push_issue(
                    &mut issues,
                    "unpaired-frequency-edges",
                    Severity::Error,
                    &pointer,
                    version,
                    "frequency lower and upper edges must be supplied together".into(),
                    None,
                );
            }
            if let (Some(lower), Some(upper)) =
                (annotation.freq_lower_edge, annotation.freq_upper_edge)
                && lower > upper
            {
                push_issue(
                    &mut issues,
                    "reversed-frequency-edges",
                    Severity::Error,
                    &pointer,
                    version,
                    "frequency lower edge exceeds the upper edge".into(),
                    None,
                );
            }
            check_extra_core_fields(
                &mut issues,
                annotation.extra_fields(),
                &pointer,
                version,
                policy,
            );
            check_extension_fields(
                &mut issues,
                annotation.extra_fields(),
                crate::extensions::ExtensionScope::Annotation,
                &pointer,
                declarations,
                registry,
                version,
            );
        }
        check_sorted(
            &mut issues,
            self.annotations()
                .iter()
                .map(|annotation| annotation.sample_start),
            "/annotations",
            version,
        );

        if let Some(extensions) = &global.extensions {
            for (index, extension) in extensions.iter().enumerate() {
                if registry
                    .is_some_and(|registry| registry.supports(&extension.name, &extension.version))
                {
                    continue;
                }
                let severity = match (extension.optional, policy) {
                    (true, _) => Severity::Warning,
                    (false, ValidationPolicy::Interoperability) => Severity::Warning,
                    (false, ValidationPolicy::StrictSupportedVersion) => Severity::Error,
                };
                let code = if extension.optional {
                    "optional-extension-unverified"
                } else {
                    "required-extension-unverified"
                };
                push_issue(
                    &mut issues,
                    code,
                    severity,
                    &format!("/global/core:extensions/{index}"),
                    version,
                    format!(
                        "{} extension {:?} has no registered semantic support",
                        if extension.optional {
                            "optional"
                        } else {
                            "required"
                        },
                        extension.name
                    ),
                    Some(
                        "Register and validate the extension before relying on its semantics."
                            .into(),
                    ),
                );
            }
        }
        let mut report = ValidationReport::new(issues);
        report.policy = Some(policy);
        report
    }
}

impl Collection {
    /// Validates this collection against SigMF rules using `policy`.
    ///
    /// Checks the collection version, that each stream's recording name is a
    /// safe relative path and is unique, that each metadata digest is 128
    /// hexadecimal characters, unknown `core:` fields, and extension
    /// declarations. Recording references are not resolved; see
    /// [`Archive::validate_collection_references`](crate::Archive::validate_collection_references).
    ///
    /// The returned report's [`policy`](ValidationReport::policy) is `None`.
    pub fn validate(&self, policy: ValidationPolicy) -> ValidationReport {
        self.validate_with_optional_registry(policy, None)
    }

    /// Validates this collection, using `registry` to verify declared extensions.
    ///
    /// Like [`validate`](Self::validate), but extensions registered in
    /// `registry` are treated as verified and their fields in the `collection`
    /// object are checked for scope and field names.
    pub fn validate_with_registry(
        &self,
        policy: ValidationPolicy,
        registry: &crate::extensions::ExtensionRegistry,
    ) -> ValidationReport {
        self.validate_with_optional_registry(policy, Some(registry))
    }

    fn validate_with_optional_registry(
        &self,
        policy: ValidationPolicy,
        registry: Option<&crate::extensions::ExtensionRegistry>,
    ) -> ValidationReport {
        let version = self.collection.version.as_str();
        let mut issues = Vec::new();
        if version != "1.2.6" {
            push_issue(
                &mut issues,
                "unsupported-version",
                match policy {
                    ValidationPolicy::Interoperability => Severity::Warning,
                    ValidationPolicy::StrictSupportedVersion => Severity::Error,
                },
                "/collection/core:version",
                version,
                format!("SigMF collection version {version} is not certified"),
                None,
            );
        }
        let mut names = HashSet::new();
        for (index, recording) in self.collection.streams.iter().enumerate() {
            let pointer = format!("/collection/core:streams/{index}");
            let name = recording.get_name();
            if name.is_empty()
                || name.starts_with('/')
                || name.contains('\\')
                || name.split('/').any(|part| part == ".." || part.is_empty())
            {
                push_issue(
                    &mut issues,
                    "invalid-recording-name",
                    Severity::Error,
                    &format!("{pointer}/name"),
                    version,
                    "recording reference name must be a non-empty relative member path".into(),
                    None,
                );
            }
            if !names.insert(name) {
                push_issue(
                    &mut issues,
                    "duplicate-recording-reference",
                    Severity::Error,
                    &format!("{pointer}/name"),
                    version,
                    format!("recording {name:?} is referenced more than once"),
                    None,
                );
            }
            let digest = recording.get_hash();
            if digest.len() != 128 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                push_issue(
                    &mut issues,
                    "invalid-metadata-digest",
                    Severity::Error,
                    &format!("{pointer}/hash"),
                    version,
                    "metadata digest must contain exactly 128 hexadecimal characters".into(),
                    None,
                );
            }
            check_extra_core_fields(
                &mut issues,
                recording.extra_fields(),
                &pointer,
                version,
                policy,
            );
        }
        check_extra_core_fields(
            &mut issues,
            self.collection.extra_fields(),
            "/collection",
            version,
            policy,
        );
        let declarations = self.collection.extensions.as_deref().unwrap_or(&[]);
        check_extension_fields(
            &mut issues,
            self.collection.extra_fields(),
            crate::extensions::ExtensionScope::Collection,
            "/collection",
            declarations,
            registry,
            version,
        );
        check_extra_core_fields(&mut issues, self.extra_fields(), "", version, policy);
        if let Some(extensions) = &self.collection.extensions {
            for (index, extension) in extensions.iter().enumerate() {
                if registry
                    .is_some_and(|registry| registry.supports(&extension.name, &extension.version))
                {
                    continue;
                }
                let severity = if extension.optional || policy == ValidationPolicy::Interoperability
                {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                let code = if extension.optional {
                    "optional-extension-unverified"
                } else {
                    "required-extension-unverified"
                };
                push_issue(
                    &mut issues,
                    code,
                    severity,
                    &format!("/collection/core:extensions/{index}"),
                    version,
                    format!(
                        "extension {:?} has no registered semantic support",
                        extension.name
                    ),
                    None,
                );
            }
        }
        ValidationReport::new(issues)
    }
}

impl crate::Archive {
    /// Checks that the archive's collection streams resolve to its recordings.
    ///
    /// Each `core:streams` entry is matched by name against the archive's
    /// [`recordings`](crate::Archive::recordings). A missing recording yields an
    /// `unresolved-recording-reference` warning (making the status
    /// [`ValidationStatus::NotChecked`]); a recording whose metadata hash
    /// differs (case-insensitively) from the reference yields a
    /// `collection-reference-digest-mismatch` error.
    ///
    /// Returns `None` if the archive has no collection.
    pub fn validate_collection_references(&self) -> Option<ValidationReport> {
        let collection = self.collection.as_ref()?;
        let version = collection.collection.version.as_str();
        let mut issues = Vec::new();
        for (index, reference) in collection.collection.streams.iter().enumerate() {
            let pointer = format!("/collection/core:streams/{index}");
            let Some(recording) = self
                .recordings
                .iter()
                .find(|recording| recording.get_name() == reference.get_name())
            else {
                push_issue(
                    &mut issues,
                    "unresolved-recording-reference",
                    Severity::Warning,
                    &format!("{pointer}/name"),
                    version,
                    format!(
                        "recording {:?} is not present in this archive",
                        reference.get_name()
                    ),
                    Some("Resolve the reference against the complete recording collection.".into()),
                );
                continue;
            };
            if !recording
                .get_hash()
                .eq_ignore_ascii_case(reference.get_hash())
            {
                push_issue(
                    &mut issues,
                    "collection-reference-digest-mismatch",
                    Severity::Error,
                    &format!("{pointer}/hash"),
                    version,
                    format!(
                        "recording {:?} does not match the referenced metadata digest",
                        reference.get_name()
                    ),
                    Some("Update the collection reference to the emitted metadata digest.".into()),
                );
            }
        }
        Some(ValidationReport::new(issues))
    }
}

fn check_index(
    issues: &mut Vec<ValidationIssue>,
    value: Option<u64>,
    pointer: &str,
    version: &str,
) {
    if value.is_some_and(|value| value > i64::MAX as u64) {
        push_issue(
            issues,
            "index-out-of-range",
            Severity::Error,
            pointer,
            version,
            "value exceeds the SigMF 1.2.6 maximum of 2^63 - 1".into(),
            None,
        );
    }
}

fn check_offset_bound(
    issues: &mut Vec<ValidationIssue>,
    sample_index: u64,
    offset: u64,
    pointer: &str,
    version: &str,
) {
    if sample_index < offset {
        push_issue(
            issues,
            "sample-index-before-offset",
            Severity::Warning,
            pointer,
            version,
            format!("sample index {sample_index} is less than core:offset {offset}"),
            None,
        );
    }
}

fn check_geolocation(
    issues: &mut Vec<ValidationIssue>,
    value: Option<&GeoJson>,
    pointer: &str,
    version: &str,
) {
    let Some(value) = value else {
        return;
    };
    let coordinates = match value {
        GeoJson::Geometry(geometry) => match &geometry.value {
            GeometryValue::Point { coordinates } => Some(coordinates),
            _ => None,
        },
        _ => None,
    };
    if !coordinates.is_some_and(|coordinates| {
        (2..=3).contains(&coordinates.len())
            && coordinates
                .as_slice()
                .iter()
                .all(|coordinate| coordinate.is_finite())
    }) {
        push_issue(
            issues,
            "invalid-geolocation",
            Severity::Error,
            pointer,
            version,
            "geolocation must be a GeoJSON Point with two or three finite coordinates".into(),
            None,
        );
    }
}

fn check_finite(
    issues: &mut Vec<ValidationIssue>,
    value: Option<f64>,
    pointer: &str,
    version: &str,
) {
    if value.is_some_and(|value| !value.is_finite()) {
        push_issue(
            issues,
            "non-finite-number",
            Severity::Error,
            pointer,
            version,
            "metadata numbers must be finite JSON numbers".into(),
            None,
        );
    }
}

fn check_sorted(
    issues: &mut Vec<ValidationIssue>,
    values: impl Iterator<Item = u64>,
    pointer: &str,
    version: &str,
) {
    let mut previous = None;
    let mut unsorted = false;
    for value in values {
        if previous.is_some_and(|previous| previous > value) {
            unsorted = true;
            break;
        }
        previous = Some(value);
    }
    if unsorted {
        push_issue(
            issues,
            "segments-not-sorted",
            Severity::Error,
            pointer,
            version,
            "segments must be ordered by core:sample_start".into(),
            Some("Use the explicit stable-sort normalization operation.".into()),
        );
    }
}

fn check_extra_core_fields(
    issues: &mut Vec<ValidationIssue>,
    extra_fields: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
    version: &str,
    policy: ValidationPolicy,
) {
    for key in extra_fields.keys().filter(|key| key.starts_with("core:")) {
        let severity = match policy {
            ValidationPolicy::Interoperability => Severity::Warning,
            ValidationPolicy::StrictSupportedVersion => Severity::Error,
        };
        push_issue(
            issues,
            "unknown-core-field",
            severity,
            &format!("{pointer}/{}", escape_pointer(key)),
            version,
            format!("unrecognized core field {key:?} is preserved but unsupported"),
            None,
        );
    }
}

fn check_extension_fields(
    issues: &mut Vec<ValidationIssue>,
    fields: &serde_json::Map<String, serde_json::Value>,
    scope: crate::extensions::ExtensionScope,
    pointer: &str,
    declarations: &[crate::global::ExtensionDeclaration],
    registry: Option<&crate::extensions::ExtensionRegistry>,
    version: &str,
) {
    for field in fields.keys() {
        let Some((namespace, _)) = field.split_once(':') else {
            continue;
        };
        if namespace == "core" {
            continue;
        }
        let field_pointer = format!("{pointer}/{}", escape_pointer(field));
        let Some(declaration) = declarations
            .iter()
            .find(|declaration| declaration.name == namespace)
        else {
            push_issue(
                issues,
                "undeclared-extension-field",
                Severity::Warning,
                &field_pointer,
                version,
                format!("extension field {field:?} has no matching core:extensions declaration"),
                Some("Declare the extension and version that define this field.".into()),
            );
            continue;
        };
        let Some(registry) = registry else {
            continue;
        };
        if !registry.supports(&declaration.name, &declaration.version) {
            continue;
        }
        if !registry.supports_scope(&declaration.name, &declaration.version, scope) {
            push_issue(
                issues,
                "extension-scope-mismatch",
                Severity::Error,
                &field_pointer,
                version,
                format!("extension field {field:?} is not valid at the {scope:?} scope"),
                None,
            );
        } else if !registry.supports_field(&declaration.name, &declaration.version, scope, field) {
            push_issue(
                issues,
                "unrecognized-extension-field",
                Severity::Warning,
                &field_pointer,
                version,
                format!("registered extension does not define field {field:?}"),
                None,
            );
        }
    }
}

fn push_issue(
    issues: &mut Vec<ValidationIssue>,
    code: &'static str,
    severity: Severity,
    pointer: &str,
    version: &str,
    message: String,
    remediation: Option<String>,
) {
    issues.push(ValidationIssue {
        code,
        severity,
        json_pointer: pointer.to_string(),
        declared_version: version.to_string(),
        message,
        remediation,
    });
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn is_sigmf_timestamp(value: &str) -> bool {
    let Some(value) = value.strip_suffix('Z') else {
        return false;
    };
    let (date, time) = match value.split_once('T') {
        Some(parts) => parts,
        None => return false,
    };
    let mut date_parts = date.split('-');
    let (Some(year), Some(month), Some(day), None) = (
        date_parts.next(),
        date_parts.next(),
        date_parts.next(),
        date_parts.next(),
    ) else {
        return false;
    };
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > days_in_month {
        return false;
    }

    let (clock, fraction) = match time.split_once('.') {
        Some((clock, fraction))
            if !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()) =>
        {
            (clock, Some(fraction))
        }
        Some(_) => return false,
        None => (time, None),
    };
    let mut clock_parts = clock.split(':');
    let (Some(hour), Some(minute), Some(second), None) = (
        clock_parts.next(),
        clock_parts.next(),
        clock_parts.next(),
        clock_parts.next(),
    ) else {
        return false;
    };
    let (Ok(hour), Ok(minute), Ok(second)) = (
        hour.parse::<u32>(),
        minute.parse::<u32>(),
        second.parse::<u32>(),
    ) else {
        return false;
    };
    let _ = fraction;
    hour <= 23 && minute <= 59 && second <= 60
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{datatype::Datatype, global::Global, meta::Metadata};
    use std::str::FromStr;

    fn metadata() -> Metadata {
        Metadata::new(
            Global::new("1.2.6", Datatype::from_str("ri16_le").unwrap()),
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn distinguishes_errors_from_recommendation_warnings() {
        let mut metadata = metadata();
        let mut lower_only = crate::annotation::Annotation::new(
            0,
            Some(0),
            Some(10.0),
            None,
            None,
            None,
            None,
            None,
        );
        lower_only.set_label("long label".into());
        metadata.add_annotation(lower_only);

        let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
        assert_eq!(report.status, ValidationStatus::Failed);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "unpaired-frequency-edges"
                    && issue.severity == Severity::Error)
        );
    }

    #[test]
    fn accepts_zero_length_annotation_and_valid_leap_second_timestamp() {
        let mut metadata = metadata();
        let mut capture = crate::capture::Capture::new(0);
        capture.set_datetime("2016-12-31T23:59:60.123Z".into());
        metadata.add_capture(capture);
        metadata.add_annotation(crate::annotation::Annotation::new(
            0,
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
        assert_eq!(
            report.status,
            ValidationStatus::Passed,
            "{:?}",
            report.issues
        );
    }

    #[test]
    fn validates_geolocation_shape_and_preserves_foreign_members() {
        let mut metadata = metadata();
        let point: GeoJson = serde_json::from_value(serde_json::json!({
            "type": "Point",
            "coordinates": [-122.0, 37.0],
            "vendor:datum": "WGS84"
        }))
        .unwrap();
        metadata.set_geolocation(point);
        assert_eq!(
            metadata
                .validate(ValidationPolicy::StrictSupportedVersion)
                .status,
            ValidationStatus::Passed
        );
        let serialized: serde_json::Value =
            serde_json::from_slice(&metadata.to_vec().unwrap()).unwrap();
        assert_eq!(
            serialized["global"]["core:geolocation"]["vendor:datum"],
            "WGS84"
        );

        let invalid: GeoJson = serde_json::from_value(serde_json::json!({
            "type": "MultiPoint",
            "coordinates": [[-122.0, 37.0]]
        }))
        .unwrap();
        metadata.set_geolocation(invalid);
        assert!(
            metadata
                .validate(ValidationPolicy::StrictSupportedVersion)
                .issues
                .iter()
                .any(|issue| issue.code == "invalid-geolocation")
        );
    }

    #[test]
    fn indices_before_core_offset_are_reported_as_recommendation_warnings() {
        let mut metadata = metadata();
        metadata.set_offset(10);
        metadata.add_capture(crate::capture::Capture::new(9));
        let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
        assert_eq!(report.status, ValidationStatus::Passed);
        assert!(report.issues.iter().any(|issue| {
            issue.code == "sample-index-before-offset" && issue.severity == Severity::Warning
        }));
    }
}
