use serde::{Deserialize, Serialize};
use sigmf::SigmfExtension;

#[derive(Deserialize, Serialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
struct MismatchedFieldNamespace {
    #[serde(rename = "other:value")]
    value: bool,
}

fn main() {}
