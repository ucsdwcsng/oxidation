# wcsng-sigmf-derive

[![crates.io](https://img.shields.io/crates/v/wcsng-sigmf-derive.svg)](https://crates.io/crates/wcsng-sigmf-derive)
[![docs.rs](https://img.shields.io/docsrs/wcsng-sigmf-derive)](https://docs.rs/wcsng-sigmf-derive)

Derive macro for typed [SigMF](https://sigmf.org) metadata extensions.

This crate is an implementation detail of
[`wcsng-sigmf`](https://crates.io/crates/wcsng-sigmf). Don't depend on it
directly. Enable the `derive` feature instead:

```toml
[dependencies]
wcsng-sigmf = { version = "0.1", features = ["derive"] }
```

```rust
use serde::{Deserialize, Serialize};
use sigmf::SigmfExtension;

#[derive(Serialize, Deserialize, SigmfExtension)]
#[sigmf(namespace = "lab", version = "1.0.0", scope = "global")]
struct LabReceiver {
    #[serde(rename = "lab:gain_db")]
    gain_db: f64,
}
```

See the [`wcsng-sigmf` documentation](https://docs.rs/wcsng-sigmf) for details.

## License

MIT
