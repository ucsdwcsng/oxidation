# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-01

### Added

- API documentation for every public item, with runnable examples.
- `sigmf::num_complex` re-export for building `SampleValue`s.
- `derive_extension` example.
- docs.rs configuration with all features enabled, plus a GitHub Pages docs workflow.

### Changed

- Reorganized the repository around the `wcsng-sigmf` crate. The derive macro
  now lives in `sigmf-derive/`.
- `wcsng-sigmf` now depends on the exact matching version of
  `wcsng-sigmf-derive`.
- Moved to the Rust 2024 edition. The minimum supported Rust version is still 1.89.

### Deprecated

- `utils::file::get_root_dir`, which returned a compile-time path that has no
  meaning for downstream crates.

## [0.1.0] - 2026-11-01

- Initial release.

[Unreleased]: https://github.com/ucsdwcsng/oxidation/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/ucsdwcsng/oxidation/releases/tag/v0.1.1
[0.1.0]: https://crates.io/crates/wcsng-sigmf/0.1.0
