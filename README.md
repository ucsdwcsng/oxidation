# wcsng-sigmf

[![crates.io](https://img.shields.io/crates/v/wcsng-sigmf.svg)](https://crates.io/crates/wcsng-sigmf)
[![docs.rs](https://img.shields.io/docsrs/wcsng-sigmf)](https://docs.rs/wcsng-sigmf)
[![CI](https://github.com/ucsdwcsng/oxidation/actions/workflows/ci.yml/badge.svg)](https://github.com/ucsdwcsng/oxidation/actions/workflows/ci.yml)
[![MSRV 1.89](https://img.shields.io/badge/MSRV-1.89-blue.svg)](https://blog.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ucsdwcsng/oxidation/blob/main/LICENSE)

A Rust library for reading, writing, and validating
[SigMF](https://sigmf.org) (Signal Metadata Format) recordings, collections,
and archives.

SigMF pairs a binary dataset of RF samples (`.sigmf-data`) with JSON metadata
(`.sigmf-meta`) that describes how the samples were captured. This crate
implements the [SigMF 1.2 specification](https://sigmf.org) with an
emphasis on correctness and on preserving data it doesn't understand.

- **Typed metadata.** `global`, `captures`, and `annotations` are strongly
  typed. Unknown and extension fields round-trip unchanged.
- **Every datatype.** Real and complex `f32`/`f64`/`i8`/`i16`/`i32`/`u8`/`u16`/`u32`
  samples in either byte order, multi-channel datasets, and non-conforming
  datasets (NCD) with capture headers and trailing bytes.
- **Streaming I/O.** Read frames by absolute sample index or iterate in
  fixed-size blocks without loading the whole dataset. Crop recordings while
  preserving sample indices.
- **Archives and collections.** Create and read `.sigmf` tar archives and
  `.sigmf-collection` files, with SHA-512 integrity checks.
- **Safe writes.** Files are written to a temporary sibling and published
  atomically. Existing files are never overwritten unless you ask.
- **Validation.** Policy-driven checks that return machine-readable issues with
  JSON pointers and remediation hints.
- **Typed extensions.** Map namespaced extension fields such as `lab:gain_db`
  to Rust structs with `#[derive(SigmfExtension)]`.

## Installation

```sh
cargo add wcsng-sigmf
```

The package is named `wcsng-sigmf`, and the library is imported as `sigmf`:

```rust
use sigmf::meta::Metadata;
```

### Feature flags

| Feature  | Description |
|----------|-------------|
| `derive` | `#[derive(SigmfExtension)]` for typed metadata extensions. |
| `schema` | JSON Schema generation and validation for extensions. |

```toml
[dependencies]
wcsng-sigmf = { version = "0.1", features = ["derive"] }
```

## Usage

### Read a recording

```rust,no_run
use sigmf::recording_file::Recording;

fn main() -> Result<(), sigmf::SigmfError> {
    let mut recording = Recording::open("capture.sigmf-meta")?;
    recording.verify_dataset_digest()?;

    let metadata = recording.metadata();
    println!("datatype:    {}", metadata.global().datatype);
    println!("sample rate: {:?}", metadata.get_sample_rate());
    println!("frames:      {}", recording.frame_count()?);

    // Stream samples in blocks of 4096 frames.
    let start = recording.first_sample_index();
    let count = recording.frame_count()?;
    for block in recording.frame_blocks(start, count, 4096)? {
        let _samples = block?;
        // process samples ...
    }
    Ok(())
}
```

### Write a recording

```rust,no_run
use sigmf::datatype::Datatype;
use sigmf::global::Global;
use sigmf::meta::Metadata;
use sigmf::num_complex::Complex;
use sigmf::recording_file::RecordingWriter;
use sigmf::utils::data::SampleValue;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut metadata = Metadata::new(
        Global::new("1.2.6", Datatype::from_str("cf32_le")?),
        Vec::new(),
        Vec::new(),
    );
    metadata.set_sample_rate(1e6);
    metadata.set_author("Jane Doe <jane@example.com>".to_string());

    let samples: Vec<SampleValue> = (0..1024)
        .map(|n| SampleValue::ComplexF32(Complex::from_polar(1.0, n as f32 * 0.1)))
        .collect();

    // Writes tone.sigmf-data and tone.sigmf-meta. core:sha512 is filled in.
    let mut writer = RecordingWriter::new("tone.sigmf-meta", metadata)?;
    writer.write_frames(&samples)?;
    let digest = writer.finish()?;
    println!("sha512: {digest}");
    Ok(())
}
```

### Archives

```rust,no_run
use sigmf::Archive;

fn main() -> Result<(), sigmf::SigmfError> {
    let archive = Archive::load("recordings.sigmf")?;
    for recording in archive.get_recordings() {
        let metadata = archive.get_meta_from_recording(&recording)?;
        println!("{}: {}", recording.get_name(), metadata.global().datatype);
    }
    Ok(())
}
```

Use `sigmf::ArchiveWriter` to create archives.

### Validation

```rust,no_run
use sigmf::meta::Metadata;
use sigmf::validation::{ValidationPolicy, ValidationStatus};

fn main() -> Result<(), sigmf::SigmfError> {
    let metadata = Metadata::from_path("capture.sigmf-meta")?;
    let report = metadata.validate(ValidationPolicy::StrictSupportedVersion);
    if report.status != ValidationStatus::Passed {
        for issue in &report.issues {
            eprintln!("{:?} {} at {}: {}", issue.severity, issue.code, issue.json_pointer, issue.message);
        }
    }
    Ok(())
}
```

### Typed extensions

With the `derive` feature:

```rust,ignore
use serde::{Deserialize, Serialize};
use sigmf::SigmfExtension;

#[derive(Debug, Serialize, Deserialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "global")]
struct LabReceiver {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
}

metadata.set_global_extension(&LabReceiver { gain_db: 30.0 })?;
let receiver: LabReceiver = metadata.global_extension()?;
```

## Examples

Runnable programs are in [`examples/`](https://github.com/ucsdwcsng/oxidation/tree/main/examples):

| Example | Description |
|---------|-------------|
| [`save_sigmf`](https://github.com/ucsdwcsng/oxidation/blob/main/examples/save_sigmf.rs) | Build metadata and write a `.sigmf` archive. |
| [`load_sigmf`](https://github.com/ucsdwcsng/oxidation/blob/main/examples/load_sigmf.rs) | List the recordings in a `.sigmf` archive. |
| [`load_recording`](https://github.com/ucsdwcsng/oxidation/blob/main/examples/load_recording.rs) | Stream every frame of a `.sigmf-meta`/`.sigmf-data` pair. |
| [`derive_extension`](https://github.com/ucsdwcsng/oxidation/blob/main/examples/derive_extension.rs) | Define, attach, and validate typed extensions (`--features derive`). |

```sh
cargo run --example save_sigmf -- /tmp/example.sigmf
cargo run --example load_sigmf -- /tmp/example.sigmf
cargo run --example derive_extension --features derive
```

## Documentation

- API reference for the latest release: <https://docs.rs/wcsng-sigmf>
- API reference for `main`: <https://ucsdwcsng.github.io/oxidation/>
- SigMF specification: <https://sigmf.org>

Build the docs locally with:

```sh
cargo doc --all-features --no-deps --open
```

## Minimum supported Rust version

Rust 1.89 or newer. Raising the MSRV is considered a minor change.

## Contributing

Issues and pull requests are welcome. Before submitting, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc --all-features
```

See [RELEASING.md](https://github.com/ucsdwcsng/oxidation/blob/main/RELEASING.md) for the release process.

## License

Licensed under the [MIT License](https://github.com/ucsdwcsng/oxidation/blob/main/LICENSE).
Developed by [WCSNG](https://wcsng.ucsd.edu) at UC San Diego.
