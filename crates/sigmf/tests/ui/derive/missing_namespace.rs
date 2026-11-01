use serde::{Deserialize, Serialize};
use sigmf::SigmfExtension;

#[derive(Deserialize, Serialize, SigmfExtension)]
#[sigmf(version = "1.0.0", scope = "capture")]
struct MissingNamespace {
    #[serde(rename = "lab:value")]
    value: bool,
}

fn main() {}
