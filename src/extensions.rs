//! Typed access to SigMF extension namespaces.
//!
//! SigMF lets applications add fields to the `global`, `captures`,
//! `annotations`, and `collection` objects under their own namespace prefix
//! (for example `antenna:gain`), declaring each namespace and version in
//! `core:extensions`. This crate preserves such fields as untyped JSON; this
//! module adds a typed layer on top:
//!
//! - [`SigmfExtension`] describes a Rust type that maps to a set of namespaced
//!   fields in one [`ExtensionScope`]. Implement it by hand, or with
//!   `#[derive(SigmfExtension)]` when the `derive` feature is enabled.
//! - Typed getters and setters read and write those fields:
//!   [`Metadata::global_extension`](crate::meta::Metadata::global_extension),
//!   [`Capture::extension`](crate::capture::Capture::extension),
//!   [`Annotation::extension`](crate::annotation::Annotation::extension),
//!   [`CollectionMetadata::extension`](crate::collection::CollectionMetadata::extension),
//!   and their `set_*`/`remove_*` counterparts.
//! - [`ExtensionRegistry`] records which extensions an application understands
//!   so that validation (e.g.
//!   [`Metadata::validate_with_registry`](crate::meta::Metadata::validate_with_registry))
//!   can verify declared extensions and their fields.
//! - [`CropExtensionHandler`] lets applications adjust extension fields when a
//!   recording is cropped.
//!
//! With the `schema` feature, [`extension_schema`] and
//! [`validate_json_schema`] generate and apply JSON Schemas for extension types.
//!
//! Setting extension fields does not modify `core:extensions`; declare the
//! extension separately, e.g. with [`SigmfExtension::declaration`].
//!
//! # Example
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use sigmf::capture::Capture;
//! use sigmf::extensions::{ExtensionScope, SigmfExtension};
//!
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct Antenna {
//!     #[serde(rename = "antenna:port")]
//!     port: u8,
//! }
//!
//! impl SigmfExtension for Antenna {
//!     const NAMESPACE: &'static str = "antenna";
//!     const VERSION: &'static str = "1.0.0";
//!     const SCOPE: ExtensionScope = ExtensionScope::Capture;
//!     const FIELDS: &'static [&'static str] = &["antenna:port"];
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut capture = Capture::new(0);
//! capture.set_extension(&Antenna { port: 2 })?;
//! assert_eq!(capture.extra_fields()["antenna:port"], 2);
//! assert_eq!(capture.extension::<Antenna>()?, Antenna { port: 2 });
//!
//! capture.remove_extension::<Antenna>()?;
//! assert!(capture.extra_fields().get("antenna:port").is_none());
//! # Ok(())
//! # }
//! ```

use crate::SigmfError;
use crate::global::ExtensionDeclaration;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The SigMF object in which an extension's fields may appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionScope {
    /// The `global` object of a `.sigmf-meta` file.
    Global,
    /// An entry of the `captures` array.
    Capture,
    /// An entry of the `annotations` array.
    Annotation,
    /// The `collection` object of a `.sigmf-collection` file.
    Collection,
}

/// Transforms application-defined fields while a recording is cropped.
pub trait CropExtensionHandler {
    /// Receives each field map that contains extension data.
    ///
    /// `sample_start` and `sample_count` refer to the unmodified capture or
    /// annotation; both are `None` for global extension fields. The crop interval
    /// uses absolute, half-open sample indices.
    fn transform_for_crop(
        &mut self,
        scope: ExtensionScope,
        sample_start: Option<u64>,
        sample_count: Option<u64>,
        crop_start: u64,
        crop_count: u64,
        fields: &mut Map<String, Value>,
    ) -> Result<(), SigmfError>;
}

/// A typed view of one SigMF extension's fields within a single scope.
///
/// The type is serialized with Serde to a JSON object whose keys are the
/// namespaced wire names (e.g. `"antenna:port"`), so fields normally use
/// `#[serde(rename = "namespace:field")]`. Every key it serializes must be
/// listed in [`FIELDS`](Self::FIELDS).
///
/// With the `derive` feature, `#[derive(sigmf::SigmfExtension)]` together with
/// `#[sigmf(namespace = "...", version = "...", scope = "...")]` implements
/// this trait, filling in the associated constants and checking at compile
/// time that every field's Serde name uses the namespace prefix. See the
/// [module documentation](self) for a manual implementation.
pub trait SigmfExtension: Serialize + DeserializeOwned {
    /// The extension namespace, i.e. the prefix before `:` in field names.
    ///
    /// Must be non-empty and must not be `"core"`. The derive macro takes it
    /// from `#[sigmf(namespace = "...")]` and additionally requires ASCII
    /// alphanumerics, `_`, or `-`.
    const NAMESPACE: &'static str;
    /// The extension version written to `core:extensions`.
    ///
    /// Must be non-empty. The derive macro takes it from
    /// `#[sigmf(version = "...")]`.
    const VERSION: &'static str;
    /// The object in which this extension's fields appear.
    ///
    /// The derive macro maps `#[sigmf(scope = "...")]` values `"global"`,
    /// `"capture"`, `"annotation"`, and `"collection"` to the corresponding
    /// [`ExtensionScope`] variant.
    const SCOPE: ExtensionScope;
    /// The complete set of namespaced wire field names this type owns.
    ///
    /// Only these keys are read by the typed getters and replaced or removed
    /// by the typed setters; other fields are left untouched. The derive macro
    /// fills this with each struct field's Serde wire name.
    const FIELDS: &'static [&'static str];

    /// Serializes `self` to a JSON value.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::Serde`] if serialization fails.
    fn to_extension_value(&self) -> Result<Value, SigmfError> {
        serde_json::to_value(self).map_err(SigmfError::Serde)
    }

    /// Deserializes an extension from a JSON value.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `value` does not match the
    /// type, e.g. because a required field is missing.
    fn from_extension_value(value: Value) -> Result<Self, SigmfError>
    where
        Self: Sized,
    {
        serde_json::from_value(value)
            .map_err(|error| SigmfError::InvalidMetadata(error.to_string()))
    }

    /// Builds the `core:extensions` entry declaring this extension.
    ///
    /// `optional` indicates whether readers may ignore the extension.
    fn declaration(optional: bool) -> ExtensionDeclaration {
        ExtensionDeclaration {
            name: Self::NAMESPACE.to_string(),
            version: Self::VERSION.to_string(),
            optional,
        }
    }
}

/// The set of extensions, by namespace and version, an application supports.
///
/// Pass a registry to
/// [`Metadata::validate_with_registry`](crate::meta::Metadata::validate_with_registry)
/// or
/// [`Collection::validate_with_registry`](crate::collection::Collection::validate_with_registry)
/// so declared extensions are treated as verified and their fields are
/// checked against each extension's [`SCOPE`](SigmfExtension::SCOPE) and
/// [`FIELDS`](SigmfExtension::FIELDS).
///
/// # Example
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use sigmf::extensions::{ExtensionRegistry, ExtensionScope, SigmfExtension};
///
/// #[derive(Serialize, Deserialize)]
/// struct Lab {
///     #[serde(rename = "lab:gain_db")]
///     gain_db: f64,
/// }
///
/// impl SigmfExtension for Lab {
///     const NAMESPACE: &'static str = "lab";
///     const VERSION: &'static str = "1.0.0";
///     const SCOPE: ExtensionScope = ExtensionScope::Global;
///     const FIELDS: &'static [&'static str] = &["lab:gain_db"];
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut registry = ExtensionRegistry::new();
/// registry.register::<Lab>()?;
/// assert!(registry.supports("lab", "1.0.0"));
/// assert!(!registry.supports("lab", "2.0.0"));
/// assert!(registry.supports_field("lab", "1.0.0", ExtensionScope::Global, "lab:gain_db"));
/// assert!(!registry.supports_scope("lab", "1.0.0", ExtensionScope::Capture));
///
/// // Registering the same namespace and version twice is an error.
/// assert!(registry.register::<Lab>().is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExtensionRegistry {
    supported: BTreeMap<(String, String), RegisteredExtension>,
}

#[derive(Debug, Clone)]
struct RegisteredExtension {
    scope: ExtensionScope,
    fields: BTreeSet<String>,
}

impl ExtensionRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers support for extension `T` at its namespace and version.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s namespace is empty or
    /// `"core"`, its version is empty, or the same namespace and version are
    /// already registered.
    pub fn register<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        let namespace = T::NAMESPACE;
        if namespace.is_empty() || namespace == "core" || T::VERSION.is_empty() {
            return Err(SigmfError::InvalidMetadata(
                "extension namespace and version must be non-empty and namespace cannot be core"
                    .into(),
            ));
        }
        let key = (namespace.to_string(), T::VERSION.to_string());
        if self.supported.contains_key(&key) {
            return Err(SigmfError::InvalidMetadata(format!(
                "extension {namespace:?} version {:?} is already registered",
                T::VERSION
            )));
        }
        self.supported.insert(
            key,
            RegisteredExtension {
                scope: T::SCOPE,
                fields: T::FIELDS.iter().map(|field| (*field).to_string()).collect(),
            },
        );
        Ok(())
    }

    /// Returns `true` if the extension `namespace` at `version` is registered.
    pub fn supports(&self, namespace: &str, version: &str) -> bool {
        self.supported
            .contains_key(&(namespace.to_string(), version.to_string()))
    }

    /// Returns `true` if the registered extension `namespace` at `version`
    /// applies to `scope`.
    ///
    /// Returns `false` if the extension is not registered.
    pub fn supports_scope(&self, namespace: &str, version: &str, scope: ExtensionScope) -> bool {
        self.supported
            .get(&(namespace.to_string(), version.to_string()))
            .is_some_and(|extension| extension.scope == scope)
    }

    /// Returns `true` if the registered extension `namespace` at `version`
    /// applies to `scope` and defines the namespaced wire name `field`.
    ///
    /// Returns `false` if the extension is not registered.
    pub fn supports_field(
        &self,
        namespace: &str,
        version: &str,
        scope: ExtensionScope,
        field: &str,
    ) -> bool {
        self.supported
            .get(&(namespace.to_string(), version.to_string()))
            .is_some_and(|extension| extension.scope == scope && extension.fields.contains(field))
    }

    /// Returns a `core:extensions` declaration for each registered extension.
    ///
    /// Declarations are sorted by namespace, then version, and are all marked
    /// as required (`optional: false`).
    pub fn declarations(&self) -> Vec<ExtensionDeclaration> {
        self.supported
            .iter()
            .map(|((name, version), _)| ExtensionDeclaration {
                name: name.clone(),
                version: version.clone(),
                optional: false,
            })
            .collect()
    }
}

/// Generates the JSON Schema for extension type `T`.
///
/// The schema describes the JSON object `T` serializes to, keyed by its
/// namespaced wire names.
///
/// # Errors
///
/// Returns [`SigmfError::Serde`] if the generated schema cannot be converted
/// to a [`Value`].
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "schema")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use serde::{Deserialize, Serialize};
/// use serde_json::json;
/// use sigmf::extensions::{
///     extension_schema, validate_json_schema, ExtensionScope, SigmfExtension,
/// };
///
/// #[derive(Serialize, Deserialize, schemars::JsonSchema)]
/// struct Lab {
///     #[serde(rename = "lab:gain_db")]
///     gain_db: f64,
/// }
///
/// impl SigmfExtension for Lab {
///     const NAMESPACE: &'static str = "lab";
///     const VERSION: &'static str = "1.0.0";
///     const SCOPE: ExtensionScope = ExtensionScope::Global;
///     const FIELDS: &'static [&'static str] = &["lab:gain_db"];
/// }
///
/// let schema = extension_schema::<Lab>()?;
/// validate_json_schema(&schema, &json!({ "lab:gain_db": 30.0 }))?;
/// assert!(validate_json_schema(&schema, &json!({ "lab:gain_db": "high" })).is_err());
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "schema"))]
/// # fn main() {}
/// ```
#[cfg(feature = "schema")]
pub fn extension_schema<T>() -> Result<Value, SigmfError>
where
    T: SigmfExtension + schemars::JsonSchema,
{
    serde_json::to_value(schemars::schema_for!(T)).map_err(SigmfError::Serde)
}

/// Validates `instance` against the JSON Schema `schema`.
///
/// For safety, schemas containing `$ref`, `$dynamicRef`, or `$recursiveRef`
/// values that are not local (`#/...`) references are rejected rather than
/// fetched.
///
/// # Errors
///
/// Returns [`SigmfError::InvalidMetadata`] if the schema contains a remote
/// reference, is not a valid JSON Schema, or `instance` does not conform to
/// it (the message lists every validation error).
#[cfg(feature = "schema")]
pub fn validate_json_schema(schema: &Value, instance: &Value) -> Result<(), SigmfError> {
    if schema_contains_remote_reference(schema) {
        return Err(SigmfError::InvalidMetadata(
            "remote JSON Schema references are disabled".into(),
        ));
    }
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| SigmfError::InvalidMetadata(format!("invalid JSON Schema: {error}")))?;
    let errors = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(SigmfError::InvalidMetadata(format!(
            "JSON Schema validation failed: {}",
            errors.join("; ")
        )));
    }
    Ok(())
}

#[cfg(feature = "schema")]
fn schema_contains_remote_reference(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            (matches!(key.as_str(), "$ref" | "$dynamicRef" | "$recursiveRef")
                && value
                    .as_str()
                    .is_some_and(|reference| !reference.starts_with("#/")))
                || schema_contains_remote_reference(value)
        }),
        Value::Array(values) => values.iter().any(schema_contains_remote_reference),
        _ => false,
    }
}

impl crate::meta::Metadata {
    /// Reads extension `T` from the fields of the `global` object.
    ///
    /// Only the keys listed in [`SigmfExtension::FIELDS`] are considered;
    /// absent keys are simply omitted before deserialization.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Global`] or the fields do not deserialize into `T`.
    pub fn global_extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Global, self.global().extra_fields())
    }

    /// Writes extension `T` into the `global` object.
    ///
    /// All of `T`'s declared [`FIELDS`](SigmfExtension::FIELDS) are first
    /// removed, then the serialized fields are inserted, so fields omitted by
    /// serialization (e.g. `None` with `skip_serializing_if`) are cleared.
    /// `core:extensions` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Global`], `extension` does not serialize to a JSON
    /// object, or it serializes a key not listed in `FIELDS`; returns
    /// [`SigmfError::Serde`] if serialization fails. The metadata is unchanged
    /// on error.
    pub fn set_global_extension<T: SigmfExtension>(
        &mut self,
        extension: &T,
    ) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Global, extension)?;
        self.update_global_extension_fields(T::FIELDS, fields);
        Ok(())
    }

    /// Removes all of extension `T`'s declared fields from the `global` object.
    ///
    /// `core:extensions` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Global`].
    pub fn remove_global_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Global)?;
        self.update_global_extension_fields(T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::capture::Capture {
    /// Reads extension `T` from this capture's fields.
    ///
    /// Only the keys listed in [`SigmfExtension::FIELDS`] are considered;
    /// absent keys are simply omitted before deserialization.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Capture`] or the fields do not deserialize into `T`.
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Capture, self.extra_fields())
    }

    /// Writes extension `T` into this capture.
    ///
    /// All of `T`'s declared [`FIELDS`](SigmfExtension::FIELDS) are first
    /// removed, then the serialized fields are inserted, so fields omitted by
    /// serialization are cleared. `core:extensions` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Capture`], `extension` does not serialize to a JSON
    /// object, or it serializes a key not listed in `FIELDS`; returns
    /// [`SigmfError::Serde`] if serialization fails. Nothing is changed on
    /// error.
    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Capture, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

    /// Removes all of extension `T`'s declared fields from this capture.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Capture`].
    pub fn remove_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Capture)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::annotation::Annotation {
    /// Reads extension `T` from this annotation's fields.
    ///
    /// Only the keys listed in [`SigmfExtension::FIELDS`] are considered;
    /// absent keys are simply omitted before deserialization.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Annotation`] or the fields do not deserialize into `T`.
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Annotation, self.extra_fields())
    }

    /// Writes extension `T` into this annotation.
    ///
    /// All of `T`'s declared [`FIELDS`](SigmfExtension::FIELDS) are first
    /// removed, then the serialized fields are inserted, so fields omitted by
    /// serialization are cleared. `core:extensions` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Annotation`], `extension` does not serialize to a JSON
    /// object, or it serializes a key not listed in `FIELDS`; returns
    /// [`SigmfError::Serde`] if serialization fails. Nothing is changed on
    /// error.
    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Annotation, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

    /// Removes all of extension `T`'s declared fields from this annotation.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Annotation`].
    pub fn remove_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Annotation)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::collection::CollectionMetadata {
    /// Reads extension `T` from this `collection` object's fields.
    ///
    /// Only the keys listed in [`SigmfExtension::FIELDS`] are considered;
    /// absent keys are simply omitted before deserialization.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Collection`] or the fields do not deserialize into `T`.
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Collection, self.extra_fields())
    }

    /// Writes extension `T` into this `collection` object.
    ///
    /// All of `T`'s declared [`FIELDS`](SigmfExtension::FIELDS) are first
    /// removed, then the serialized fields are inserted, so fields omitted by
    /// serialization are cleared. `core:extensions` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Collection`], `extension` does not serialize to a JSON
    /// object, or it serializes a key not listed in `FIELDS`; returns
    /// [`SigmfError::Serde`] if serialization fails. Nothing is changed on
    /// error.
    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Collection, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

    /// Removes all of extension `T`'s declared fields from this `collection` object.
    ///
    /// # Errors
    ///
    /// Returns [`SigmfError::InvalidMetadata`] if `T`'s scope is not
    /// [`ExtensionScope::Collection`].
    pub fn remove_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Collection)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, Map::new());
        Ok(())
    }
}

fn get_extension_fields<T: SigmfExtension>(
    scope: ExtensionScope,
    source: &Map<String, Value>,
) -> Result<T, SigmfError> {
    ensure_scope::<T>(scope)?;
    let fields = T::FIELDS
        .iter()
        .filter_map(|field| {
            source
                .get(*field)
                .map(|value| ((*field).to_string(), value.clone()))
        })
        .collect();
    T::from_extension_value(Value::Object(fields))
}

fn extension_fields<T: SigmfExtension>(
    scope: ExtensionScope,
    extension: &T,
) -> Result<Map<String, Value>, SigmfError> {
    ensure_scope::<T>(scope)?;
    let Value::Object(fields) = extension.to_extension_value()? else {
        return Err(SigmfError::InvalidMetadata(
            "extension must serialize as an object".into(),
        ));
    };
    if fields
        .keys()
        .any(|field| !T::FIELDS.contains(&field.as_str()))
    {
        return Err(SigmfError::InvalidMetadata(
            "extension serialization produced an undeclared field".into(),
        ));
    }
    Ok(fields)
}

fn ensure_scope<T: SigmfExtension>(scope: ExtensionScope) -> Result<(), SigmfError> {
    if T::SCOPE != scope {
        return Err(SigmfError::InvalidMetadata(
            "extension scope does not match the metadata object".into(),
        ));
    }
    Ok(())
}

fn replace_extension_fields(
    destination: &mut Map<String, Value>,
    declared_fields: &[&str],
    replacement: Map<String, Value>,
) {
    for field in declared_fields {
        destination.remove(*field);
    }
    destination.extend(replacement);
}
