# SigMF Library Modernization Proposal

**Status:** Implementation underway; gate status is recorded in Section 17  
**Audience:** Library maintainers, Rust contributors, and downstream application developers  
**Specification baseline:** SigMF 1.2.6  
**Prepared:** October 31, 2026

## 1. Executive Summary

The `sigmf` crate should evolve from an archive-oriented utility into a
standards-conscious Rust library for the complete SigMF recording lifecycle:
constructing metadata, loading recordings, inspecting samples, modifying
documents, validating constraints, serializing metadata, saving datasets, and
packaging collections and archives.

The central architectural decision is to separate **metadata**, **sample
storage**, and **packaging**. A recording is not an archive, a collection entry
is not an opened recording, and parsing JSON is not proof that a dataset is
valid. Each concept should have a distinct type and an explicit contract.

The recommended implementation provides:

- Strongly typed core metadata with preservation of unknown extension fields.
- Idiomatic, fallible constructors and borrowed accessors.
- Correct real and complex sample codecs with explicit byte order.
- Streaming and seekable I/O without mandatory whole-dataset allocation.
- Structured validation reports, version policies, and extension support.
- Safe metadata editing and explicit sample transformation operations.
- Reliable persistence, integrity verification, and collection hash updates.
- Optional extension derives, schema generation, archive support, and adapters.
- A staged migration that preserves access to historical recordings without
  silently accepting or producing malformed SigMF.

This document began as a proposal. Core metadata, recording I/O, persistence,
archive indexing, extension support, and codec benchmarks have since been
implemented. The acceptance gates below distinguish verified behavior from
remaining certification, resource-measurement, and platform work; unverified
guarantees must not be inferred from implemented APIs.

## 2. Scope and Design Principles

### 2.1 Supported workflows

The target library should support:

1. Loading metadata from UTF-8 JSON strings, byte slices, readers, and files.
2. Opening conforming metadata/data pairs from either file's path.
3. Creating recordings from typed samples or already encoded bytes.
4. Loading metadata-only distributions without pretending a dataset exists.
5. Describing supported non-conforming datasets and reporting unsupported ones.
6. Editing global fields, capture segments, annotations, and extension fields.
7. Querying metadata by sample interval, frequency interval, or namespace.
8. Reading and writing typed sample blocks, ranges, and channels.
9. Saving metadata independently or publishing an entire recording.
10. Reading, modifying, validating, and writing collections and `.sigmf` archives.
11. Computing and verifying dataset and metadata hashes.
12. Defining typed extensions through traits, derives, and generated schemas.

### 2.2 Governing principles

| Principle | Required consequence |
| --- | --- |
| Standards first | Wire names, datatype grammar, file rules, and segment semantics follow a pinned SigMF release. |
| Preserve information | Unsupported metadata survives ordinary load/edit/save operations. |
| No hidden data loss | Failed decoding, unsupported layouts, and failed edits return errors rather than empty data. |
| Explicit cost | Full allocation, conversion, hashing, indexing, and copying are visible API choices. |
| Explicit authority | The library distinguishes upstream requirements from local policy and recommendations. |
| Small dependable core | Filesystem, archives, macros, and runtime adapters do not become inseparable from metadata. |
| Rust-native ergonomics | Use ownership, borrowing, iterators, `Result`, and meaningful types rather than clone-heavy getters. |
| Deterministic behavior | Ordering and serialization policies are documented; parallel execution does not reorder results. |

### 2.3 Non-goals

The core library is not a DSP framework, plotting toolkit, acquisition driver,
database, remote schema downloader, or automatic converter for arbitrary
container formats. Resampling, filtering, demodulation, and application-specific
signal interpretation belong in downstream libraries.

Async I/O, memory mapping, Python bindings, object storage, and a CLI are useful
future integrations, not prerequisites for a correct first release.

## 3. Current-State Assessment

The existing implementation contains useful domain separation between global
metadata, captures, annotations, and collections. Its principal limitation is
that the public workflow is organized around a tar archive and repeated
materialization of its contents.

| Existing surface | Observed limitation | Modernization direction |
| --- | --- | --- |
| [`SigMF` and `Builder`](../../crates/sigmf/src/lib.rs) | Recording identity and archive paths are coupled; construction and extension checks can panic. | Separate opened recordings, archive readers, and fallible writers. |
| [`SmfMeta`](../../crates/sigmf/src/meta.rs) | Many accessors clone; metadata hashing reserializes and unwraps results. | Borrowed views, explicit edit operations, and byte-defined hashing. |
| [`SmfGlobal`](../../crates/sigmf/src/global.rs) | Datatype components are strings and optional pieces; accepted values and getters are not fully aligned. | A closed core datatype representation with exact parsing. |
| [`SmfAnnotation`](../../crates/sigmf/src/annotation.rs) | Core fields do not have namespaced Serde renames. | Correct wire names and explicit historical-input migration. |
| [`SmfCapture`](../../crates/sigmf/src/capture.rs) | Capture geolocation is absent; optional fields serialize through the default `Option` behavior. | Complete baseline fields and intentional omission rules. |
| [`Sample utilities`](../../crates/sigmf/src/utils/data.rs) | Native-memory casts do not implement declared endianness; real byte paths halve the element count. | Shared, tested codecs with complete-frame checks. |
| [`Archive utilities`](../../crates/sigmf/src/utils/file.rs) | Lookups reopen and scan archives; metadata lookup uses content hashes; paths and header ownership are environment-specific. | One-pass indexing, qualified member names, portable headers, and explicit limits. |
| [`Collection model`](../../crates/sigmf/src/collection.rs) | References and opened recordings share terminology; extension information is not preserved comprehensively. | Dedicated reference types and namespace-aware preservation. |
| [`Hash tests`](../../crates/sigmf/src/utils/hash.rs) | Fixtures depend on a hard-coded checkout path. | Self-contained fixtures and portable temporary directories. |
| [`Save example`](../../crates/sigmf/examples/save_sigmf.rs) | Demonstrates archive-first construction and default-then-set metadata. | Examples for plain recording creation, editing, streaming, and archives. |

Additional concerns directly affect reliability: `trim` converts sample-read
failures into empty vectors, archive saving serializes access through a mutex,
and ordinary derived metadata structs do not retain unrecognized fields.
These should inform the redesign rather than be replicated as compatibility
requirements.

## 4. Specification Baseline and Compatibility Policy

### 4.1 Authoritative sources

At preparation time, the official latest release endpoint identifies
**v1.2.6**. Use that release as the initial implementation baseline:

- [Published SigMF specification](https://sigmf.org/).
- [SigMF 1.2.6 release](https://github.com/sigmf/SigMF/releases/tag/v1.2.6).
- [Pinned metadata schema](https://github.com/sigmf/SigMF/blob/v1.2.6/sigmf-schema.json).
- [Pinned collection schema](https://github.com/sigmf/SigMF/blob/v1.2.6/collection-schema.json).
- [Pinned supplemental specification text](https://github.com/sigmf/SigMF/blob/v1.2.6/additional_content.md).
- [Canonical extension definitions](https://github.com/sigmf/SigMF/tree/v1.2.6/extensions).

The published website is mutable. Implementation fixtures and semantic rule
references must use release-pinned material, retaining attribution and applicable
license notices. Rust crate versions, SigMF specification versions, and
extension versions are three independent version domains.

### 4.2 Rules that shape the API

- Metadata has a `global` object and `captures` and `annotations` arrays. All
  three members are required, even when arrays are empty.
- `core:datatype` and `core:version` are required global fields. Schema defaults
  do not authorize their omission.
- Complex components are stored in I/Q order. Multiple channels are interleaved
  by sample index, with each channel's sample occupying its own position.
- Both captures and annotations are sorted by `core:sample_start`. Annotation
  ties are permitted; their relative ordering is not defined by the standard.
- Capture fields are scoped to their segment. Frequency and timestamp values
  must not silently carry into subsequent segments.
- An omitted annotation count has capture-dependent meaning: it applies through
  the end of the corresponding capture, not necessarily the entire dataset.
- Metadata-only distributions are not complete SigMF recordings.
- `core:dataset`, capture headers, and trailing bytes describe non-conforming
  layouts; interpreting their metadata does not establish recording compliance.
- `core:sha512` hashes dataset-file bytes. A collection reference hashes the
  referenced metadata-file bytes.
- An extension with `optional: false` must be supported before semantic use.
- Recording references may use objects or legacy two-element tuples in 1.x.
  Objects are recommended; tuples are scheduled for removal in SigMF 2.0.

### 4.3 Upstream inconsistencies must remain visible

The pinned upstream artifacts are not interchangeable validators:

1. The collection schema models stream entries as tuples, while specification
   prose recommends objects and allows both.
2. A schema property description reverses the meaning of extension `optional`,
   while the normative extension table states that `false` means required.
3. Some schema regular expressions are not fully anchored and do not enforce
   the complete datatype or digest grammar.
4. Global `core:offset` describes absolute indexing, while capture descriptions
   and examples also describe dataset-relative sample starts.

Maintain a versioned discrepancy register in the implementation's conformance
fixtures. Preserve the upstream schemas unchanged; implement supplemental
semantic checks and document any separately published library interoperability
schema. An upstream-schema result must not be relabeled as full conformance.

For disputed index semantics, the first release must document its interpretation,
support it with pinned upstream evidence and reference-tool fixtures, and seek
upstream clarification. Do not expose a permanent coordinate-conversion
contract based on an unexamined assumption. Section 8 defines the internal
coordinate separation needed regardless of that resolution.

### 4.4 Read, validate, and write policies

Offer three distinct actions:

- **Parse:** Decode syntax and retain fields without claiming semantic support.
- **Validate:** Report structural, semantic, extension, and optional storage issues.
- **Write:** Apply an explicit output version and validation policy before emission.

An unsupported future version can be retained for inspection, but it must not
receive a supported-version compliance result. Do not silently rewrite
`core:version` on load. Initially certify 1.2.6; list earlier versions as
uncertified until version-specific fixtures establish their behavior.

## 5. Architecture and Package Boundaries

### 5.1 Recommended package structure

Start with one runtime crate and one optional procedural-macro crate:

```text
crates/
  sigmf/
    src/
      metadata/
      datatype.rs
      validation/
      extension/
      sample/
      recording/
      collection/
      archive/
      error.rs
  sigmf-derive/
    src/
```

These are proposed modules, not a requirement to create every directory
immediately. Keep cohesive code together until size justifies subdivision.
Do not split the core into many mutually dependent crates prematurely.

### 5.2 Layering

```text
Application workflows and optional adapters
                 |
 Recording / Collection / Archive operations
                 |
       Sample codecs and layout mapping
                 |
 Metadata / Datatype / Extensions / Validation
                 |
              Serde JSON
```

Metadata parsing and serialization must work without opening files. Semantic
validation takes explicit context; it must not unexpectedly traverse a directory
or hash a dataset. Storage-backed validation is a separate operation.

### 5.3 Feature policy

| Feature | Intended responsibility |
| --- | --- |
| Default metadata support | Core types, JSON serialization, preservation, and semantic validation. |
| `fs` | Path-based recording and collection operations. |
| `archive` | POSIX tar reading and writing. |
| `derive` | Re-export extension derives from `sigmf-derive`. |
| `schema` | Schema generation and optional upstream-schema validation. |
| `mmap` | Explicit, constrained mapped dataset access. |
| `async` | Runtime adapter selected only after demonstrated demand. |

Decide the default inclusion of `fs` before publishing the new API; ordinary
desktop use should remain convenient. Avoid speculative `no_std` guarantees.
Document the supported Rust version and test feature combinations. Progress
bars, parallel execution, GeoJSON convenience libraries, and platform user
lookups must not be mandatory dependencies merely because the old crate uses them.

## 6. Metadata Representation and Serialization

### 6.1 Public domain types

Use names that express the standard directly:

- `Metadata`: global information, captures, annotations, and extension objects.
- `Global`: recording-wide core fields and additional namespaced fields.
- `Capture`: segment start and optional capture-scoped information.
- `Annotation`: start, optional count, frequency bounds, and descriptive fields.
- `Collection`: collection-wide metadata and ordered recording references.
- `RecordingRef`: a name, metadata digest, and permitted extension fields.
- `ExtensionDeclaration`: namespace, declared version, and optionality.

Constrained scalar wrappers should include sample indices, counts, channel
counts, supported format versions, UTC timestamps, and SHA-512 digests.
Use wrappers only where they establish a real invariant. A description remains
a string, not an unnecessary abstraction.

A metadata builder requires datatype and version and starts with empty segment
arrays. Do not provide `Default` for types whose required fields cannot have a
meaningful valid default.

### 6.2 Wire mapping and numeric fidelity

Use Serde derives for ordinary fields with explicit `core:*` renames and
`skip_serializing_if = "Option::is_none"` where absence is permitted.
Do not emit `null` for omitted optional core numbers or strings unless the
target field actually permits it.

Use `u64` for nonnegative indices internally, but enforce field-specific schema
limits, including the 1.2.6 limit of `2^63 - 1` on relevant index/count fields.
Never route integer indices through `f64`; values above `2^53` must survive.
Checked arithmetic is required for byte positions, range ends, and sizes.

Validate finite JSON metadata numbers and the baseline field limits. Do not
reject NaN or infinity in IEEE-754 dataset samples simply because JSON metadata
cannot represent them.

Timestamps must follow SigMF's UTC `Z` form, support permitted fractional
precision, and explicitly handle leap-second syntax. Preserve the original
timestamp text when a time-library conversion cannot retain all precision.
GeoJSON locations must be Points, with longitude/latitude ordering and
permitted foreign members preserved.

### 6.3 Unknown-field preservation

Retain unrecognized fields at all extensible scopes, including global, captures,
annotations, permitted top-level extension objects, collection fields,
recording-reference fields, and GeoJSON foreign members.

Known fields and extra-field storage must never serialize the same key twice.
Reject attempts to insert a known core field through the extra-field API.
Unknown `core:*` fields are preserved for inspection but diagnosed according to
the declared-version policy; they are not automatically approved extensions.

Parsing duplicate JSON member names must return a path-aware error. A generic
`serde_json::Value` intermediate that has already discarded duplicates cannot
provide that guarantee; implement duplicate detection at the visitor layer.

### 6.4 Preservation is not byte identity

Provide separate guarantees:

- **Semantic round trip:** Unknown values and absent/present distinctions survive.
- **Original-byte retention:** An unmodified loaded document can expose or copy its
  original bytes, useful for collection hash verification.
- **Formatted serialization:** Edited metadata is emitted as compact or pretty JSON
  with documented ordering and newline policy.

Ordinary serialization is not byte-preserving. Whitespace, number spelling, and
object-key order can change without changing metadata meaning.

Deterministic JSON is useful for reproducibility but must not be called canonical
JSON unless a specific canonicalization standard is implemented. Do not derive
SigMF metadata hashes from an invented semantic-normalization algorithm.

## 7. Editing and Querying

### 7.1 Access and mutation

Favor borrowed access such as `global()`, `captures()`, and `annotations()`.
Avoid allocating clones unless the caller asks for ownership.

Use checked setters for local invariants and an edit transaction for changes
that involve multiple fields:

```rust
// Proposed API.
let mut document = Metadata::from_slice(&json_bytes)?;
let mut edit = document.edit();
edit.set_description("Reviewed field recording");
edit.add_annotation(
    Annotation::builder(SampleIndex::new(4096)?)
        .sample_count(SampleCount::new(2048)?)
        .label("burst")
        .build()?,
)?;
let report = edit.validate(&context)?;
edit.commit(&report)?;
```

A transaction edits a working copy or change set. Validation failure leaves the
original unchanged. A report must be tied to the revision and validation context
it checked; later edits invalidate it, and `commit` must detect stale reports.
Direct construction of a draft remains possible without claiming validation.

### 7.2 Operations with precise semantics

- Add, replace, and remove capture or annotation entries.
- Clear optional fields rather than requiring replacement with dummy values.
- Insert, retrieve, and remove namespaced extension values.
- Stable-sort segments as an explicit normalization operation.
- Query captures covering a sample and annotations intersecting a range.
- Query optional frequency intervals without guessing RF/baseband context.
- Rename recordings through a storage operation that updates dependent references.
- Produce a structured diff for metadata review and audit trails.

Do not automatically merge captures or carry fields forward. A query view may
apply the documented global geolocation fallback, but that does not change
capture-segment scoping rules.

### 7.3 Cropping and transformation

Cropping changes sample data and metadata together. Require an explicit policy
for index rebasing, origin preservation, partially intersecting annotations,
capture-boundary metadata, timestamps, and extension fields containing indices.

Frequency conversion, integer scaling, channel selection, and datatype
conversion must be named operations with declared rounding, clipping, overflow,
and provenance behavior. Unsupported extension transformations block an operation
or require explicit user acknowledgement; silently stale extension coordinates
are unacceptable.

Retain the old archive `trim` behavior only under an unambiguous name such as
`retain_recordings_with_minimum_frames`. It is collection filtering, not cropping.
Use validated layout counts rather than decoding entire recordings to count them.

## 8. Datatypes, Sample Coordinates, and Dataset Layout

### 8.1 Closed core datatype model

Represent the complete core format through a validated enum or private-field
value type. Parse exact strings through `FromStr`; expose `Display` and Serde
string conversion from the same representation.

The 1.2.6 core grammar comprises:

- Real and complex domains.
- Signed and unsigned 8-bit components without endian suffixes.
- Signed and unsigned 16- and 32-bit components with `_le` or `_be`.
- IEEE-754 32- and 64-bit components with `_le` or `_be`.

That produces **28 core datatype spellings**. Signed or unsigned 64-bit integer
samples are not baseline core formats; the old parser's partial acceptance
must not become a new conformance promise.

Reject prefixes, trailing suffixes, missing required endianness, and illegal
byte-type suffixes. Preserve unsupported extension datatype strings in an
inspection representation, but do not decode them without a registered codec.

### 8.2 Define sample units explicitly

Use separate vocabulary for:

- **Component:** One scalar, such as the I component of `ci16_le`.
- **Channel sample:** One real value or complex pair for one channel.
- **Frame:** All channel samples at one recorded time index.
- **Dataset-relative frame:** Position in the stored sequence.
- **Metadata sample index:** The coordinate expressed by SigMF metadata.
- **Source-stream index:** The coordinate described by `core:global_index`.
- **Byte offset:** Position in the physical dataset file.

For a conforming dataset:

```text
components_per_channel_sample = 1 for real, 2 for complex
bytes_per_channel_sample = component_bytes * components_per_channel_sample
bytes_per_frame = bytes_per_channel_sample * num_channels
frame_count = dataset_bytes / bytes_per_frame
byte_offset(relative_frame) = relative_frame * bytes_per_frame
```

The remainder must be zero. The default channel count is one; no frame-count
API should accidentally report scalar component count.

Coordinate conversion is a single shared, tested layout service. Its
version-specific mapping between metadata indices and dataset-relative frames
must resolve the `core:offset` discrepancy identified in Section 4.3. Source
stream indices must never be substituted for file offsets.

### 8.3 Capture boundaries and non-conforming layouts

Build a segment table from ordered capture starts, header byte counts, datatype,
channel count, total byte length, and trailing bytes. A logical read crossing a
header must split into physical reads and skip non-sample bytes.

Validate that headers and trailers fit, remaining bytes form complete frames,
and all arithmetic is checked. Generic support covers layouts described by
SigMF metadata; it does not parse arbitrary WAV, HDF5, or proprietary containers.

Empty captures have the baseline's implied capture semantics without forcing a
new serialized entry. Out-of-range captures receive diagnostics and can be
ignored by a processing view, as recommended by application-compliance guidance.
Validation must not silently delete them from the source document.

## 9. Sample Reading and Writing

### 9.1 Two complementary access levels

Provide raw byte access and typed sample access:

```rust
// Proposed API; typed decoding requires a matching stored datatype.
let mut recording = Recording::open("session.sigmf-meta")?;
let mut reader = recording.samples::<num_complex::Complex<i16>>()?;
for block in reader.blocks(16_384)? {
    let block = block?;
    consume(block.frames());
}
```

A dynamic reader returns a homogeneous typed block enum rather than an enum
for every scalar or sample. This avoids per-element discriminants and matches
the recording's single datatype.

Typed reading preserves stored numeric values. Requesting a different numeric
type invokes a separate conversion API, not an implicit cast.

### 9.2 Codec contract

- Decode using explicit `from_le_bytes` or `from_be_bytes` paths.
- Encode using the corresponding explicit byte methods.
- Check complete scalars, complex pairs, and multi-channel frames.
- Reject incomplete final input with its byte offset and expected frame size.
- Handle short reads, interrupted I/O, and chunk boundaries between components.
- Preserve every real 8-bit sample; never halve real element counts.
- Preserve I/Q and channel ordering exactly.
- State integer-to-float normalization and saturation policies only in conversions.

Start with safe scalar codecs. A cast-based fast path is optional and requires
alignment, length, endian, lifetime, and representation checks. If those
conditions fail, use the correct decoder rather than panic.

Memory mapping is not universally safe: external mutation or truncation can
invalidate assumptions. Any mapped API must describe its platform hazards and
backing-file stability requirements; do not advertise a blanket safe zero-copy
guarantee.

### 9.3 Capability-aware readers

Readers over `Read` support sequential traversal. Readers over `Read + Seek`
support ranges. File-backed readers may add positional access. A sequential
archive reader does not promise random access.

Prefer generics for normal Rust I/O composition and introduce dynamic dispatch
only at genuine runtime-selection boundaries. Iterator errors remain visible
as `Result` items; iteration must not terminate successfully on a read error.

Typed writers validate their sample type against metadata. Raw writers check
frame boundaries and exact byte counts, finalize explicitly, and report flush
and integrity-update failures.

## 10. Loading, Saving, and Publication Guarantees

### 10.1 Load surfaces

Separate constructors for `Metadata`, `Recording`, `Collection`, and `Archive`.
A convenience detector may classify supported file suffixes, but must return
the detected kind rather than force every input into a `SigMF` object.

Use `AsRef<Path>` and `PathBuf` for filesystem operations. Resolve dataset names
relative to the metadata file, not the process working directory. Preserve
non-UTF-8 filesystem paths internally and validate JSON filenames separately.

Expose metadata loading without opening data, and allow a metadata-only result
whose sample-reading method returns a clear unavailable-dataset error.

### 10.2 Save operations

Distinguish:

- Serialize to a caller-provided writer.
- Save metadata to a new path.
- Replace one metadata file atomically where supported.
- Copy a recording to a new location.
- Publish newly encoded samples and metadata.
- Export a collection or build an archive.

Default to no overwrite. Replacement requires explicit options. Serialize and
validate before altering existing targets whenever practical.

### 10.3 Crash consistency

For a single file, write a sibling temporary file, finish writing, optionally
sync it, and rename or replace through a platform-appropriate implementation.
Durability policy must distinguish flushing, file syncing, and directory syncing.

A metadata/data pair cannot generally be committed through one atomic rename.
Do not promise pairwise atomicity. Prefer staging an entire new recording
directory and publishing that directory when the destination model permits.
For replacement inside an existing directory, specify commit ordering, recovery
state, and concurrent-reader limitations. Metadata-last publication reduces some
risks but does not make replacing both files atomic.

Saving to the source path must never truncate a file that is still being read.
Failures return explicit context and clean up only temporary files owned by the
operation. Finalization errors cannot be left solely to `Drop`.

### 10.4 Integrity and collection dependencies

Compute dataset hashes incrementally over the actual dataset-file bytes,
including non-sample headers or trailers when hashing an NCD file.
Maintain separate names for dataset digests and metadata digests.

When metadata is rewritten, a collection's reference hash changes even if the
dataset does not. Serialize final metadata once, hash those exact bytes, and
write those same bytes. If dataset content changes, update `core:sha512` before
final metadata serialization.

A standalone save cannot discover every external collection. Return the new
metadata digest and document which in-scope references were updated. Never
claim to have repaired references outside the opened collection.

## 11. Collections and Archives

### 11.1 Collections

Keep ordered `RecordingRef` values separate from opened recordings. Loading a
collection should not eagerly open or decode every referenced dataset.

Accept object and exact two-element tuple forms under the 1.x policy. Preserve
which form was loaded when requested; normalized output prefers objects.
Because the baseline collection schema does not fully match that recommendation,
report schema and semantic results separately.

Validate names, digest syntax, extension declarations, duplicate identities,
reference resolution, and optional integrity checks. Preserve reference order:
it may convey channel or stream association. Do not infer synchronization,
alignment, or common sample rate merely because recordings share a collection.

### 11.2 Archive reading

Support POSIX.1-2001 tar with the `.sigmf` suffix. Build a directory-qualified
member index in one pass where seeking is available; do not identify metadata
by repeated archive-wide content hashing.

Respect arbitrary allowed directory structures. Two recordings with the same
basename in different directories are distinct. A compliant archive contains
at least one recording and no more than one top-level collection file.

For non-seekable inputs, offer sequential entry access or explicit bounded
spooling; do not disguise temporary storage as zero-cost seeking.

### 11.3 Archive writing and defensive handling

Use deterministic member order when requested. Portable archive headers must
not depend on local user databases; timestamps and ownership need an explicit
preserve-versus-reproducible policy.

Reject unsafe extraction paths, escaping links, duplicate conflicting members,
unsupported special files, and excessive resource consumption. Reader traversal
without extraction should be the default. Extraction must enforce destination
containment beyond a simple string-prefix check.

Preserve additional permitted archive members during ordinary rewrites or
require an explicit drop policy. Renaming members updates dataset names and
in-scope collection references; hash recomputation follows actual serialized
bytes. Archive finalization and trailer writes must succeed before reporting
completion.

## 12. Validation and Diagnostics

### 12.1 Validation layers

| Layer | Examples | Needs dataset I/O? |
| --- | --- | --- |
| Syntax | UTF-8, JSON grammar, duplicate keys. | No |
| Structure | Required objects, field types, core wire names. | No |
| Core semantics | Datatype grammar, limits, ordering, paired frequency bounds. | No |
| Extensions | Declaration support, scope, version, extension constraints. | No, unless defined by an explicit extension check |
| Layout | Frame alignment, capture headers, trailers, mapped bounds. | Size and possibly reads |
| Integrity | Dataset SHA-512 and collection metadata hashes. | Yes |
| Packaging | Naming, directory relationships, archive contents. | Yes |

A structured report contains issue code, severity, JSON Pointer, optional
source location, declared version, explanation, and remediation guidance.
Return multiple independent issues when safe; avoid follow-on noise from
already invalid foundational fields.

Reports distinguish **passed**, **failed**, and **not checked**. Missing datasets,
unregistered optional schemas, and unrequested hashes must not appear verified.

### 12.2 Policy distinctions

Provide an interoperability inspection policy and a supported-version strict
policy. Unknown optional namespaces remain readable and preserved. Strict
validation can report that their conformance is unverified without blocking
inspection.

Unknown required extensions allow raw document inspection but block a validated
recording view. Registering only a JSON shape schema is insufficient when the
extension also defines a new dataset format or processing semantics.

Required rules and recommendations receive different diagnostics. For example,
a long annotation label violates a recommendation, while supplying only one
frequency edge violates an explicit requirement. Zero-length annotations are
not forbidden by the baseline's count minimum.

### 12.3 Repair

Repair is opt-in and returns a change report. Candidate repairs include stable
segment sorting, known historical field renames, and tuple-to-object conversion.
Never invent missing datatype, timestamps, sample counts, or extension meanings.
Do not automatically erase conflicting or out-of-range metadata.

## 13. Extensions, Derive Macros, and Schemas

### 13.1 Runtime extension model

Every namespaced field remains available as raw JSON. Typed access is an
additional view, not a replacement for preservation.

An extension registry identifies namespace, supported version range, allowed
scope, field schema, semantic validators, and any required codec or
transformation capabilities. Duplicate namespace/version registrations are
explicit errors.

Typed extension updates modify only their declared fields, leaving unrecognized
fields in that namespace intact unless replacement is explicitly requested.
Removing an extension declaration while its fields remain must be diagnosed.

### 13.2 Derive contract

Use one focused derive, `SigmfExtension`, that composes with ordinary Serde
derives and optional schema support:

```rust
// Proposed derive syntax; "lab" is a private example, not a canonical extension.
#[derive(Debug, serde::Serialize, serde::Deserialize, sigmf::SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
struct LabCapture {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,

    #[serde(rename = "lab:device_id", skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
}
```

The derive generates field descriptors, trait implementation, typed field
extraction/update helpers, and schema integration when enabled. It must not
generate competing Serde implementations.

Required compile-time diagnostics:

- Missing or malformed namespace/version/scope attributes.
- Reserved `core` namespace.
- Duplicate wire names or fields outside the declared namespace.
- Unsupported enum or container shapes in the initial derive.
- Conflicting Serde renames, flattening, tagging, or custom serializers whose
  wire shape cannot be established.
- Unmet trait bounds, with errors pointing to the affected field.

Handle renamed dependencies, visibility, generics, and lifetime bounds
deliberately. Support advanced shapes only with compile-pass and compile-fail
fixtures. Runtime validation remains necessary for value constraints and
cross-field relationships.

### 13.3 Schema strategy

Adopt JSON Schema Draft 2020-12 for baseline compatibility. Evaluate an existing
Rust schema generator before writing custom machinery, and verify its actual
dialect support and handling of Serde attributes.

Keep three artifacts distinct:

1. Unmodified, release-pinned upstream schemas.
2. Library model/interoperability schemas with explicit differences.
3. User extension schemas generated from declared extension types.

Generated schemas must reflect namespaced wire keys, required fields,
optional omission behavior, numeric constraints, and unknown-field policy.
Schema validation alone cannot establish capture ordering, byte layout,
sample availability, or digest correctness.

Use offline schema registration by default. Do not follow arbitrary remote
`$ref` URLs during parsing; an explicit resolver must enforce schemes, resource
limits, trust policy, and cycle handling.

A generated JSON schema does not replace the SigMF extension's normative
Markdown definition. Document that authors must specify semantics, units,
scope, versioning, and required behavior separately.

### 13.4 Declarative macros

Do not add a broad macro DSL to the first release. Builders and Rust struct
syntax should remain the primary construction mechanisms.

After real usage demonstrates a need, small `annotation!` or namespaced-field
helpers may reduce repetition. They must evaluate expressions once, preserve
ordinary `Result` error handling, and provide useful spans. Avoid a second
metadata language that duplicates the builder API and hides fallibility.

## 14. Error Model and Resource Limits

Use a public, non-exhaustive error enum with typed causes:

- I/O failure with operation and path or archive-member context.
- JSON syntax or structural failure with source position and JSON Pointer.
- Invalid datatype, unsupported version, or unsupported required extension.
- Invalid sample range, truncated frame, or arithmetic overflow.
- Missing dataset or unresolved recording reference.
- Integrity mismatch with expected and actual digest.
- Publication conflict, failed finalization, or unsafe archive member.
- Resource limit exceeded with the configured limit and observed amount.

Preserve `std::error::Error::source` for underlying failures. Validation reports
are distinct from transport errors: malformed content may produce a report,
while a failed file read produces an I/O error.

Library code must not print warnings or progress bars unconditionally. Return
diagnostics and provide optional callbacks or tracing integration where useful.
Cancellation must propagate as a visible outcome, not a successful partial save.

Configure finite metadata size, nesting, archive entry count, total extracted
bytes, and buffered/spooled bytes. Enforce limits during consumption rather
than trusting declared lengths. Initial numeric defaults must be chosen from
fixture and workload measurements and documented before release; no method
may silently disable them.

## 15. Performance and Concurrency

Correctness precedes optimization. Establish the following cost contracts:

- Opening metadata does not allocate memory proportional to dataset size.
- Reading `k` frames uses memory proportional to the configured block plus
  metadata/layout state, not the complete dataset.
- Archive indexing is one scan; qualified lookup does not rehash every member.
- Counting frames requires layout information, not sample decoding.
- Raw copying and hashing use bounded buffers.
- Dynamic dispatch and enums operate per homogeneous block, not per sample.

Benchmark metadata parse/serialize, typed decode/encode, range reads, hashing,
collection resolution, and archive indexing on fixed reproducible workloads.
Measure throughput, allocation count, peak resident memory, and scaling.
Publish measured hardware, build profile, feature set, and input sizes.

Do not invent throughput guarantees before benchmarking. A concrete bounded
memory gate should compare a 100 MiB and a 10 GiB sparse test dataset with the
same block size: dataset-size growth must not cause proportional buffering.
Accounting must separate library buffers from mapping-related resident pages
and operating-system caches.

Parallelize independent work only after evidence demonstrates a benefit.
Preserve output order and propagate failures deterministically. Writing to one
tar stream remains ordered; placing a shared writer behind a mutex does not
make it meaningfully parallel.

## 16. Verification Strategy

### 16.1 Required test families

| Family | Required coverage |
| --- | --- |
| Wire fixtures | Every baseline core field, correct namespacing, required arrays, absent optional values. |
| Datatype grammar | All 28 core spellings and malformed prefixes, suffixes, widths, and endian combinations. |
| Codec golden bytes | All formats; known LE/BE bytes; signed boundaries; float special values; I/Q and channel order. |
| Streaming | One-byte reads, interrupted reads, arbitrary chunk splits, incomplete frames, explicit cancellation. |
| Preservation | Unknown fields at every extensible scope, foreign members, large integers, no duplicate output keys. |
| Coordinates | Multi-channel frames, capture boundaries, offsets, source-stream discontinuities, annotation count omission. |
| NCD layouts | Multiple headers, trailers, crossing reads, impossible sizes, overflow. |
| Validation | Severity distinctions, paired frequency edges, sorted arrays, UTC timestamps, unsupported extensions. |
| Persistence | No overwrite, same-path protection, short writes, finalization failure, cleanup, recovery behavior. |
| Integrity | Hash original metadata bytes; update hashes after edits; verify exact emitted dataset bytes. |
| Collections | Object and tuple entries, preserved order, missing members, duplicate names, extension fields. |
| Archives | Nested directories, repeated basenames, additional members, conflicting entries, extraction containment. |
| Macros and schemas | Compile-pass/fail cases, renamed crate imports, requiredness, wire-name snapshots, dialect validation. |
| Portability | Linux, macOS, Windows; non-UTF-8 paths where supported; no user-database assumptions. |

Property tests should prove encode/decode round trips and layout arithmetic.
Fuzz JSON visitors, datatype parsing, archive traversal, and sample block
decoders with the invariant that untrusted input does not panic.
Use Miri and targeted tooling for any newly introduced unsafe fast paths.

### 16.2 Interoperability

Run representative emitted recordings through a pinned version of the official
[Python SigMF implementation](https://github.com/sigmf/sigmf-python). Load fixtures
emitted by that implementation in Rust. Record the tool version and separate
known schema discrepancies from actual interoperability failures.

Reference-tool behavior is evidence, not a substitute for specification review.
Where tools and prose disagree, retain a reproducible fixture and a documented
decision instead of accepting whichever implementation is easiest to mimic.

### 16.3 CI and documentation

Use the workspace's existing Cargo tooling as the foundation:

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

After optional features are introduced, add focused no-default-feature,
all-feature, and minimum-supported-Rust jobs. Documentation examples become
compiled examples or doctests as their APIs are implemented. All tests must
use embedded or repository fixtures and isolated temporary directories.

## 17. Delivery Roadmap and Acceptance Gates

### Phase 1: Standards-correct metadata foundation

Implement core models, exact datatype parsing, complete field renames,
preservation, duplicate-key errors, serialization policies, and validation
reports. Establish the discrepancy register and historical-input fixtures.

**Status:** Core parsing, preservation, validation, and all 28 datatype spellings
are implemented and covered by tests. Release-pinned upstream schema fixtures
and a complete discrepancy register remain outstanding; do not claim upstream
schema conformance.

**Gate:** All core wire fixtures pass; all 28 datatype spellings round-trip;
unknown metadata survives; invalid required fields produce path-aware errors;
no unsupported document receives a conformance claim.

### Phase 2: Recording I/O and sample codecs

Implement path-relative resolution, recording/data separation, safe streaming
codecs, frame accounting, range access, and supported NCD layouts.

**Status:** File-backed range access, bounded blocks, generic short-read sample
decoding, complete-frame checks, and NCD read accounting are implemented.
Large/sparse-dataset memory measurements remain outstanding.

**Gate:** Golden-byte tests cover every supported format; one-byte streaming
works; multi-channel counts are correct; index interpretation is documented;
bounded-memory tests pass; incomplete frames never disappear silently.

### Phase 3: Editing and reliable persistence

Implement checked edits, revision-bound validation, metadata saves, recording
publication, incremental hashing, and explicit transformation policies.

**Status:** Transactional metadata edits, atomic metadata persistence, and
no-overwrite recording publication are implemented. Dataset and metadata files
are published separately, with metadata last; interruption may leave an orphan
dataset, and crop/conversion transformations remain intentionally unimplemented
until their extension-coordinate policies are defined.

**Gate:** Failed edits preserve the original; failed saves preserve existing
targets according to the documented guarantee; hashes match exact emitted
bytes; sample and metadata transformations remain consistent.

### Phase 4: Collections and archives

Implement reference forms, lazy resolution, one-pass archive indexing,
portable writers, extra-member preservation, and defensive extraction.

**Status:** Ordered references, optional legacy tuple-form preservation,
directory-qualified member indexes, digest synchronization, extra-member
preservation, and path/special-entry rejection are implemented. Archive
extraction is not provided; archive reading does not extract members.

**Gate:** Nested archives round-trip without identity collisions; stream order
is stable; member loss requires explicit policy; collection digests are updated;
finalization errors are observable.

### Phase 5: Extension ergonomics and schemas

Implement extension registry, manual trait integration, derive macros, schema
generation, and extension authoring documentation.

**Status:** Global, capture, annotation, and collection typed access/update,
scope-aware registration, derive diagnostics, and optional Draft 2020-12 schema
generation are implemented. Consumer-crate compile-fail snapshots and a
normative extension-authoring template remain outstanding.

**Gate:** Handwritten and derived extensions behave identically; compile-fail
diagnostics are useful; generated schemas match wire behavior; required
extensions block unsupported semantic use.

### Phase 6: Performance and integrations

Benchmark and optimize demonstrated bottlenecks. Add mapped access, async
adapters, CLI utilities, or language bindings only with concrete downstream needs.

**Status:** A release-mode codec benchmark and independently buildable feature
combinations are implemented and verified. Memory-gate measurements and an
MSRV policy remain open; mapped, async, CLI, and language-binding surfaces are
deferred because no concrete downstream requirement is established.

**Gate:** Optimizations preserve codec fixtures and resource contracts; unsafe
code has documented invariants and dedicated validation; optional features
remain independently buildable.

## 18. Migration and Release Strategy

Treat the redesign as an explicitly announced breaking release. Although the
crate is currently `0.1.0`, downstream users still deserve migration guidance,
behavioral release notes, and a supported transition path.

| Existing API | Proposed direction |
| --- | --- |
| `SmfMeta` | `Metadata` with borrowed views and an edit API. |
| `SmfGlobal`, `SmfCapture`, `SmfAnnotation` | Standards-named domain types and checked builders. |
| `SmfDataType::new` with optional components | Exact `Datatype::from_str` or checked constructors. |
| `SigMF` archive container | `Archive` and separately opened `Recording` values. |
| `SmfRecording` name/hash pair | `RecordingRef`; not a sample-storage handle. |
| `Builder::from_recording` | `Recording::open` with file-relative resolution. |
| Archive `Builder` | Explicit `ArchiveWriter` with fallible creation and finish. |
| `get_samples_from_name` | Typed or dynamic block readers, with an explicit allocating convenience method. |
| Metadata `hash()` | Original-byte or explicit serialized-byte digest operation. |
| Archive `trim` | Fallible minimum-frame filtering without sample allocation. |

Aliases are acceptable only when behavior and types genuinely match. Do not
retain misleading APIs through silent wrappers.

Historical unnamespaced annotations require an opt-in import policy that
renames recognized fields and returns a migration report. Conflicting old and
new names must error rather than choose a winner. Unknown fields are retained.
New output always uses the selected supported wire format.

Provide runnable migration examples for plain pairs, archives, metadata-only
inputs, extension preservation, and collections. Replace hard-coded local
paths with command-line arguments or temporary example outputs when those
examples are revised.

## 19. Decisions Required Before API Stabilization

| Decision | Recommendation | Timing |
| --- | --- | --- |
| Initial certified specification | Pin SigMF 1.2.6; certify older versions individually. | Phase 1 |
| Offset interpretation | Resolved against SigMF 1.2.6: indices are absolute; file-relative frame = index − `core:offset`. | Implemented and tested |
| Unknown-field storage | Preserve semantic JSON values; retain original bytes separately. | Phase 1 |
| Schema discrepancies | Keep upstream artifacts intact; publish supplemental checks with provenance. | Phase 1 |
| Filesystem default feature | Prefer convenient desktop use without making archives mandatory. | Before public API release |
| Derive/schema dependency | Reuse a proven schema generator after dialect and Serde compatibility tests. | Phase 5 |
| Save durability | Single-file saves sync the file and (on Unix) parent directory; pair publication is dataset-first/metadata-last and is not crash-atomic across both files. | Implemented; platform matrix pending |
| Resource defaults | Metadata/collection members are capped at 1 MB; sample blocks are caller-bounded. Archive allocation and sparse-dataset measurements remain open. | Partially implemented |
| Mapped and async APIs | Defer until workload and safety requirements justify them. | Phase 6 |

The release is ready when it can accurately describe what it preserves, what it
validates, what it decodes, and what it guarantees on save. A larger API surface
is not the objective; a coherent, extensible, and trustworthy recording lifecycle
is.
