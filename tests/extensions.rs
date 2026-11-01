#![cfg(feature = "derive")]

use serde::{Deserialize, Serialize};
use serde_json::json;
use sigmf::{
    SigmfExtension,
    extensions::{ExtensionRegistry, SigmfExtension as SigmfExtensionTrait},
    global::{ExtensionDeclaration, Global},
    meta::Metadata,
    validation::{ValidationPolicy, ValidationStatus},
};
use std::str::FromStr;

extern crate sigmf as custom_sigmf;

#[derive(Debug, Deserialize, PartialEq, Serialize, SigmfExtension)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "global")]
struct LabExtension {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
    #[serde(rename = "lab:device_id", skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
}

#[derive(Deserialize, Serialize, SigmfExtension)]
#[sigmf(
    namespace = "renamed",
    version = "2.0.0",
    scope = "capture",
    crate = "custom_sigmf"
)]
struct RenamedDependencyExtension {
    #[serde(rename = "renamed:marker")]
    marker: bool,
}

#[derive(Debug, Deserialize, Serialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
struct LabCaptureExtension {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
}

#[derive(Debug, Deserialize, Serialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "annotation")]
struct LabAnnotationExtension {
    #[serde(rename = "lab:kind")]
    kind: String,
}

#[derive(Debug, Deserialize, Serialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "collection")]
struct LabCollectionExtension {
    #[serde(rename = "lab:site")]
    site: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ManualLabCaptureExtension {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
}

impl SigmfExtensionTrait for ManualLabCaptureExtension {
    const NAMESPACE: &'static str = "lab";
    const VERSION: &'static str = "1.0.0";
    const SCOPE: sigmf::extensions::ExtensionScope = sigmf::extensions::ExtensionScope::Capture;
    const FIELDS: &'static [&'static str] = &["lab:gain_db"];
}

#[test]
fn derive_and_registry_support_typed_global_extensions() {
    let mut metadata = Metadata::from_slice(
        br#"{
            "global": {
                "core:version": "1.2.6",
                "core:datatype": "ri16_le",
                "lab:gain_db": 6.5,
                "lab:future_field": "preserve me"
            },
            "captures": [],
            "annotations": []
        }"#,
    )
    .unwrap();
    let extension = metadata.global_extension::<LabExtension>().unwrap();
    assert_eq!(
        extension,
        LabExtension {
            gain_db: 6.5,
            device_id: None
        }
    );

    metadata
        .set_global_extension(&LabExtension {
            gain_db: 8.0,
            device_id: Some("receiver-1".into()),
        })
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&metadata.to_vec().unwrap()).unwrap();
    assert_eq!(value["global"]["lab:gain_db"], 8.0);
    assert_eq!(value["global"]["lab:device_id"], "receiver-1");
    assert_eq!(value["global"]["lab:future_field"], "preserve me");

    let mut global = Global::new(
        "1.2.6",
        sigmf::datatype::Datatype::from_str("ri16_le").unwrap(),
    );
    global.set_extensions(vec![ExtensionDeclaration {
        name: LabExtension::NAMESPACE.into(),
        version: LabExtension::VERSION.into(),
        optional: false,
    }]);
    let declared = Metadata::new(global, Vec::new(), Vec::new());
    assert_eq!(
        declared
            .validate(ValidationPolicy::StrictSupportedVersion)
            .status,
        ValidationStatus::Failed
    );

    let mut registry = ExtensionRegistry::new();
    registry.register::<LabExtension>().unwrap();
    assert!(registry.supports_scope(
        LabExtension::NAMESPACE,
        LabExtension::VERSION,
        sigmf::extensions::ExtensionScope::Global
    ));
    assert!(!registry.supports_scope(
        LabExtension::NAMESPACE,
        LabExtension::VERSION,
        sigmf::extensions::ExtensionScope::Capture
    ));
    assert_eq!(
        declared
            .validate_with_registry(ValidationPolicy::StrictSupportedVersion, &registry)
            .status,
        ValidationStatus::Passed
    );
    assert!(registry.register::<LabExtension>().is_err());
}

#[test]
fn validation_rejects_registered_extensions_at_the_wrong_scope() {
    let metadata = Metadata::from_slice(
        br#"{
            "global": {
                "core:version": "1.2.6",
                "core:datatype": "ri16_le",
                "core:extensions": [{"name":"lab","version":"1.0.0","optional":false}]
            },
            "captures": [{"core:sample_start":0,"lab:gain_db":2.0}],
            "annotations": []
        }"#,
    )
    .unwrap();
    let mut registry = ExtensionRegistry::new();
    registry.register::<LabExtension>().unwrap();
    let report =
        metadata.validate_with_registry(ValidationPolicy::StrictSupportedVersion, &registry);
    assert_eq!(report.status, ValidationStatus::Failed);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "extension-scope-mismatch")
    );
}

#[test]
fn extension_values_keep_unknown_fields_when_updated() {
    let extension = LabExtension {
        gain_db: 4.0,
        device_id: None,
    };
    assert_eq!(
        extension.to_extension_value().unwrap(),
        json!({"lab:gain_db": 4.0})
    );
    let renamed = RenamedDependencyExtension { marker: true };
    assert_eq!(
        renamed.to_extension_value().unwrap(),
        json!({"renamed:marker": true})
    );
}

#[test]
fn typed_extensions_work_at_capture_annotation_and_collection_scopes() {
    let mut capture = sigmf::capture::Capture::new(0);
    capture
        .insert_extra("lab:future", json!("preserve"))
        .unwrap();
    capture
        .set_extension(&LabCaptureExtension { gain_db: 12.0 })
        .unwrap();
    assert_eq!(
        capture.extension::<LabCaptureExtension>().unwrap().gain_db,
        12.0
    );
    assert_eq!(
        capture
            .extension::<ManualLabCaptureExtension>()
            .unwrap()
            .gain_db,
        12.0
    );
    assert_eq!(
        LabCaptureExtension { gain_db: 12.0 }
            .to_extension_value()
            .unwrap(),
        ManualLabCaptureExtension { gain_db: 12.0 }
            .to_extension_value()
            .unwrap()
    );
    capture.remove_extension::<LabCaptureExtension>().unwrap();
    assert_eq!(capture.extra_fields()["lab:future"], "preserve");

    let mut annotation =
        sigmf::annotation::Annotation::new(0, None, None, None, None, None, None, None);
    annotation
        .set_extension(&LabAnnotationExtension {
            kind: "burst".into(),
        })
        .unwrap();
    assert_eq!(
        annotation
            .extension::<LabAnnotationExtension>()
            .unwrap()
            .kind,
        "burst"
    );

    let mut collection = sigmf::collection::CollectionMetadata::new("1.2.6", Vec::new());
    collection
        .set_extension(&LabCollectionExtension {
            site: "north".into(),
        })
        .unwrap();
    assert_eq!(
        collection
            .extension::<LabCollectionExtension>()
            .unwrap()
            .site,
        "north"
    );
    assert!(capture.extension::<LabAnnotationExtension>().is_err());
}

#[cfg(feature = "schema")]
#[test]
fn generated_extension_schema_uses_namespaced_wire_keys() {
    let schema = sigmf::extensions::extension_schema::<LabExtension>().unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(schema["properties"].get("lab:gain_db").is_some());
    assert!(schema["properties"].get("lab:device_id").is_some());
    assert_eq!(schema["required"], json!(["lab:gain_db"]));
    sigmf::extensions::validate_json_schema(&schema, &json!({"lab:gain_db": 6.5})).unwrap();
    assert!(
        sigmf::extensions::validate_json_schema(&schema, &json!({"lab:gain_db": "not a number"}))
            .is_err()
    );
}

#[cfg(feature = "schema")]
#[test]
fn schema_validation_rejects_remote_references() {
    let schema = json!({"$ref": "https://example.invalid/schema.json"});
    assert!(sigmf::extensions::validate_json_schema(&schema, &json!({})).is_err());
}
