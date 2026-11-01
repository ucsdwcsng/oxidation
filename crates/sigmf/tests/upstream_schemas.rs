#![cfg(feature = "schema")]

use serde_json::{json, Value};

const METADATA_SCHEMA: &str =
    include_str!("fixtures/upstream/sigmf-1.2.6/sigmf-schema.json");
const COLLECTION_SCHEMA: &str =
    include_str!("fixtures/upstream/sigmf-1.2.6/collection-schema.json");

fn validate(schema: &str, instance: &Value) -> Result<(), sigmf::SigmfError> {
    sigmf::extensions::validate_json_schema(&serde_json::from_str(schema)?, instance)
}

#[test]
fn pinned_upstream_schemas_compile_and_validate_representative_documents() {
    let metadata = json!({
        "global": {
            "core:version": "1.2.6",
            "core:datatype": "ri16_le"
        },
        "captures": [{"core:sample_start": 0}],
        "annotations": []
    });
    validate(METADATA_SCHEMA, &metadata).unwrap();
    assert!(validate(METADATA_SCHEMA, &json!({
        "global": {"core:version": "1.2.6"},
        "captures": [],
        "annotations": []
    }))
    .is_err());

    let collection_tuple = json!({
        "collection": {
            "core:version": "1.2.6",
            "core:streams": [["recording", "digest"]]
        }
    });
    validate(COLLECTION_SCHEMA, &collection_tuple).unwrap();
    assert!(validate(
        COLLECTION_SCHEMA,
        &json!({
            "collection": {
                "core:version": "1.2.6",
                "core:streams": [{"name": "recording", "hash": "digest"}]
            }
        })
    )
    .is_err());
}

#[test]
fn pinned_metadata_schema_discrepancies_remain_observable() {
    let incomplete_datatype = json!({
        "global": {
            "core:version": "1.2.6",
            "core:datatype": "ri16_le-with-suffix"
        },
        "captures": [],
        "annotations": []
    });
    validate(METADATA_SCHEMA, &incomplete_datatype).unwrap();

    let schema: Value = serde_json::from_str(METADATA_SCHEMA).unwrap();
    let optional_description = schema["properties"]["global"]["properties"]["core:extensions"]
        ["items"]["anyOf"][0]["properties"]["optional"]["description"]
        .as_str()
        .unwrap();
    assert!(optional_description.contains("REQUIRED to parse"));
}
