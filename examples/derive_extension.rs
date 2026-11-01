//! Define a typed SigMF extension with `#[derive(SigmfExtension)]`, attach it to
//! metadata, and validate the result against a registry of supported extensions.
//!
//! ```sh
//! cargo run --example derive_extension --features derive
//! ```

use serde::{Deserialize, Serialize};
use sigmf::{
    SigmfExtension,
    capture::Capture,
    datatype::Datatype,
    extensions::{ExtensionRegistry, SigmfExtension as _},
    global::Global,
    meta::Metadata,
    validation::ValidationPolicy,
};
use std::error::Error;
use std::str::FromStr;

/// Receiver settings stored in the `global` object under the `lab:` namespace.
#[derive(Debug, Serialize, Deserialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "global")]
struct LabReceiver {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
    #[serde(rename = "lab:device_id", skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
}

/// Per-capture antenna information in a separate `antenna:` namespace.
#[derive(Debug, Serialize, Deserialize, SigmfExtension)]
#[sigmf(namespace = "antenna", version = "1.0.0", scope = "capture")]
struct AntennaPort {
    #[serde(rename = "antenna:port")]
    port: u8,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut global = Global::new("1.2.6", Datatype::from_str("cf32_le")?);
    global.set_extensions(vec![
        LabReceiver::declaration(false),
        AntennaPort::declaration(true),
    ]);
    let mut metadata = Metadata::new(global, Vec::new(), Vec::new());
    metadata.set_sample_rate(1e6);

    metadata.set_global_extension(&LabReceiver {
        gain_db: 30.0,
        device_id: Some("usrp-x300-23".into()),
    })?;

    let mut capture = Capture::new(0);
    capture.set_extension(&AntennaPort { port: 1 })?;
    metadata.add_capture(capture);

    let receiver: LabReceiver = metadata.global_extension()?;
    println!("receiver: {receiver:?}");

    // Registering the extensions lets strict validation check their fields.
    let mut registry = ExtensionRegistry::new();
    registry.register::<LabReceiver>()?;
    registry.register::<AntennaPort>()?;
    let report =
        metadata.validate_with_registry(ValidationPolicy::StrictSupportedVersion, &registry);
    println!("validation: {:?}", report.status);
    for issue in &report.issues {
        println!("  [{:?}] {}: {}", issue.severity, issue.code, issue.message);
    }

    println!("{}", String::from_utf8(metadata.to_vec_pretty()?)?);
    Ok(())
}
