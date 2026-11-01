# SigMF API Migration Guide

The modern API separates metadata (`Metadata`), an opened sample pair
(`Recording`), collection references (`RecordingRef`), and tar packaging
(`Archive`). Parsing metadata does not open a dataset or certify its validity.

## API name changes

| Older name | Replacement |
| --- | --- |
| `SmfMeta` | `Metadata` |
| `SmfGlobal` | `Global` |
| `SmfCapture` | `Capture` |
| `SmfAnnotation` | `Annotation` |
| `SmfDataType` | `Datatype` |
| `SmfRecording` | `RecordingRef` |
| `SigMF` | `Archive` |
| `Builder` | `ArchiveWriter` |

Core datatype parsing is exact and fallible:

```rust
use sigmf::datatype::Datatype;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let datatype = Datatype::from_str("ci16_le")?;
    println!("{datatype}");
    Ok(())
}
```

## Opening a plain recording

Paths are resolved relative to the metadata file, not the process working
directory. Sample indices passed to `read_frames` and `frame_blocks` are
absolute SigMF indices; `core:offset` is translated to a file-relative frame.
Use blocks for bounded allocation:

```rust
use sigmf::recording_file::Recording;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut recording = Recording::open("capture.sigmf-meta")?;
    let start = recording.first_sample_index();
    let count = recording.frame_count()?;
    for block in recording.frame_blocks(start, count, 4096)? {
        let samples = block?;
        // Process this block before requesting the next one.
        println!("block sample values: {}", samples.len());
    }
    Ok(())
}
```

`Recording::open_metadata_only` opens the metadata without opening the dataset.
Dataset reads and frame counts return an explicit error when no dataset exists.

## Writing a plain recording

`RecordingWriter` encodes complete channel frames, computes SHA-512 from the
written dataset bytes, and publishes metadata after the dataset. It does not
overwrite existing paths and does not synthesize NCD capture headers or trailer
bytes. A process interruption between the two file publications can leave an
orphan dataset, but cannot publish metadata that refers to an unwritten dataset.

```rust
use sigmf::recording_file::RecordingWriter;
use sigmf::utils::data::SampleValue;
use sigmf::{
    datatype::Datatype,
    global::Global,
    meta::Metadata,
};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = Metadata::new(
        Global::new("1.2.6", Datatype::from_str("ri16_le")?),
        Vec::new(),
        Vec::new(),
    );
    let mut writer = RecordingWriter::new("output.sigmf-meta", metadata)?;
    writer.write_frames(&[
        SampleValue::RealI16(12),
        SampleValue::RealI16(-3),
    ])?;
    let dataset_sha512 = writer.finish()?;
    println!("dataset SHA-512: {dataset_sha512}");
    Ok(())
}
```

To replace metadata alone, use `Metadata::replace_atomic`. `Metadata::save_new`
uses no-overwrite publication; both methods return the digest of the exact
serialized metadata bytes. A metadata-only distribution can be parsed and
validated without a data file.

## Reading and writing archives

Archive recording names are directory-qualified. `Archive::get_samples_from_name`
is an allocating convenience method; use plain `Recording` block reads when
bounded access is required.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = sigmf::Archive::load("input.sigmf")?;
    for recording in archive.get_recordings() {
        let metadata = archive.get_meta_from_recording(&recording)?;
        println!("{}: {:?}", recording.get_name(), metadata.global().datatype);
    }
    archive.save("output.sigmf")?;
    Ok(())
}
```

Create archives with `ArchiveWriter::new`, add recordings with `add`, and call
`build` to finalize and publish. `add` computes the dataset digest and rejects
bytes that disagree with a supplied `core:sha512`. Use `add_meta` for a
metadata-only member. Archive rewriting preserves additional regular members
and updates in-archive collection references to the emitted metadata digest.

## Collections

`RecordingRef` is an ordered name/hash reference, not an opened recording.
Collection loading does not eagerly open or decode referenced datasets.

```rust
use sigmf::collection::Collection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let collection = Collection::from_slice(&std::fs::read("group.sigmf-collection")?)?;
    let normalized = collection.to_vec()?;
    let original_stream_shapes = collection.to_vec_preserving_stream_forms()?;
    println!(
        "normalized bytes: {}, preserved bytes: {}",
        normalized.len(),
        original_stream_shapes.len()
    );
    Ok(())
}
```

The default serializer normalizes legacy two-element stream tuples to objects.
`to_vec_preserving_stream_forms` preserves tuple-versus-object form when that
distinction matters. `Archive::validate_collection_references` reports unresolved
external references as not checked and in-archive digest mismatches as errors.

## Preserving and editing extensions

Unknown JSON members remain available through `extra_fields` and survive normal
serialization. Typed extension APIs are an additional view. Enable the optional
derive feature with `sigmf = { features = ["derive"] }`; enable schema support
separately with `features = ["schema"]`.

```rust
#[derive(serde::Serialize, serde::Deserialize, sigmf::SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
struct LabCapture {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut capture = sigmf::capture::Capture::new(0);
    capture.set_extension(&LabCapture { gain_db: 6.0 })?;
    Ok(())
}
```

Use `Metadata::validate` or `validate_with_registry` before relying on semantic
conformance. Reports distinguish failed validation from information that was
not checked, such as unsupported or undeclared extension fields.
