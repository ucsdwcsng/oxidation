# Releasing

This repository publishes two crates to crates.io:

| Package              | Library        | Path            |
|----------------------|----------------|-----------------|
| `wcsng-sigmf`        | `sigmf`        | `.`             |
| `wcsng-sigmf-derive` | `sigmf_derive` | `sigmf-derive/` |

Both crates share one version, `[workspace.package] version` in the root
`Cargo.toml`. `wcsng-sigmf` pins the derive crate to that exact version (`=x.y.z`
in `[workspace.dependencies]`), so update both when you bump the version.

## Checklist

1. Update `version` and the `sigmf-derive` dependency version in `Cargo.toml`.
2. Add a section to `CHANGELOG.md`.
3. Run the validation suite (Rust 1.89 or newer):

   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-targets --all-features
   cargo test --workspace --doc --all-features
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
   cargo package --workspace
   ```

   `cargo package` verifies the packaged sources and writes the `.crate`
   archives to `target/package/`. For an uncommitted release candidate, add
   `--allow-dirty`. Prefer a clean, committed worktree for publishing.

4. Commit, then tag and push:

   ```sh
   git tag v0.1.1
   git push origin main v0.1.1
   ```

## Publishing from GitHub Actions

The [publish workflow](.github/workflows/publish.yml) runs when a `v*` tag is
pushed. It checks that the tag matches both crate versions, runs the validation
suite, uploads the `.crate` archives as a build artifact, and publishes
`wcsng-sigmf-derive` before `wcsng-sigmf`.

It needs a repository secret named `CRATES_IO_TOKEN` (**Settings > Secrets and
variables > Actions**). The secret holds a crates.io API token with publish
rights for both crates. The secret is exposed as `CARGO_REGISTRY_TOKEN` only to
the publish steps. Never commit the token.

Published versions can't be overwritten. If a release partly succeeds, publish
the remaining crate by hand instead of rerunning the whole workflow:

```sh
cargo publish -p wcsng-sigmf
```

## Publishing manually

Publish in dependency order. Cargo waits for the derive crate to become
available in the registry before it publishes the main crate:

```sh
cargo publish -p wcsng-sigmf-derive
cargo publish -p wcsng-sigmf
```

## Documentation

- docs.rs builds the documentation for each published version automatically,
  with all features enabled (see `[package.metadata.docs.rs]`).
- The [docs workflow](.github/workflows/docs.yml) publishes the API docs for
  `main` to GitHub Pages. Under **Settings > Pages**, set **Source** to
  **GitHub Actions** once.
