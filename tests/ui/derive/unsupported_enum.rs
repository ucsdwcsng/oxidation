use serde::{Deserialize, Serialize};
use sigmf::SigmfExtension;

#[derive(Deserialize, Serialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
enum Unsupported {
    Value,
}

fn main() {}
