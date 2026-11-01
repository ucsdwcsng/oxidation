//! Lower-level helpers used by the rest of the crate.
//!
//! * [`data`] encodes and decodes raw sample bytes.
//! * [`hash`] computes SHA-512 digests as used by `core:sha512`.
//! * [`file`](mod@file) builds and searches tar members of `.sigmf` archives.
//! * [`debug`] contains diagnostic output helpers.

pub mod data;
pub mod debug;
pub mod file;
pub mod hash;
