use crate::global::ExtensionDeclaration;
use crate::SigmfError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionScope {
    Global,
    Capture,
    Annotation,
    Collection,
}

pub trait SigmfExtension: Serialize + DeserializeOwned {
    const NAMESPACE: &'static str;
    const VERSION: &'static str;
    const SCOPE: ExtensionScope;
    const FIELDS: &'static [&'static str];

    fn to_extension_value(&self) -> Result<Value, SigmfError> {
        serde_json::to_value(self).map_err(SigmfError::Serde)
    }

    fn from_extension_value(value: Value) -> Result<Self, SigmfError>
    where
        Self: Sized,
    {
        serde_json::from_value(value)
            .map_err(|error| SigmfError::InvalidMetadata(error.to_string()))
    }

    fn declaration(optional: bool) -> ExtensionDeclaration {
        ExtensionDeclaration {
            name: Self::NAMESPACE.to_string(),
            version: Self::VERSION.to_string(),
            optional,
        }
    }
}

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
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn supports(&self, namespace: &str, version: &str) -> bool {
        self.supported
            .contains_key(&(namespace.to_string(), version.to_string()))
    }

    pub fn supports_scope(&self, namespace: &str, version: &str, scope: ExtensionScope) -> bool {
        self.supported
            .get(&(namespace.to_string(), version.to_string()))
            .is_some_and(|extension| extension.scope == scope)
    }

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

#[cfg(feature = "schema")]
pub fn extension_schema<T>() -> Result<Value, SigmfError>
where
    T: SigmfExtension + schemars::JsonSchema,
{
    serde_json::to_value(schemars::schema_for!(T)).map_err(SigmfError::Serde)
}

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
    pub fn global_extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Global, self.global().extra_fields())
    }

    pub fn set_global_extension<T: SigmfExtension>(
        &mut self,
        extension: &T,
    ) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Global, extension)?;
        self.update_global_extension_fields(T::FIELDS, fields);
        Ok(())
    }

    pub fn remove_global_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Global)?;
        self.update_global_extension_fields(T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::capture::Capture {
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Capture, self.extra_fields())
    }

    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Capture, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

    pub fn remove_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Capture)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::annotation::Annotation {
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Annotation, self.extra_fields())
    }

    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Annotation, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

    pub fn remove_extension<T: SigmfExtension>(&mut self) -> Result<(), SigmfError> {
        ensure_scope::<T>(ExtensionScope::Annotation)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, Map::new());
        Ok(())
    }
}

impl crate::collection::CollectionMetadata {
    pub fn extension<T: SigmfExtension>(&self) -> Result<T, SigmfError> {
        get_extension_fields::<T>(ExtensionScope::Collection, self.extra_fields())
    }

    pub fn set_extension<T: SigmfExtension>(&mut self, extension: &T) -> Result<(), SigmfError> {
        let fields = extension_fields::<T>(ExtensionScope::Collection, extension)?;
        replace_extension_fields(self.extension_fields_mut(), T::FIELDS, fields);
        Ok(())
    }

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
