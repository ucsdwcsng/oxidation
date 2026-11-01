# `oxidation`
> A rust utility library.
> 
> While developing rust crates there are many methods that I have to rewrite over and over again. This repo serves as a common space for anyone to add useful utility functions.

## Crates

- `wcsng-sigmf`: Read, write, and validate SigMF recordings, metadata, and archives.
  Optional features: `derive` for extension macros and `schema` for schema validation.
- `wcsng-sigmf-derive`: Derive macros for SigMF metadata extensions.
- `oxidation`: The private utility workspace root, not published to crates.io.

The Rust library names remain `sigmf` and `sigmf_derive`. To use SigMF with
extension macros:

```toml
[dependencies]
sigmf = { package = "wcsng-sigmf", version = "0.1.0", features = ["derive"] }
```

## Release packaging

Rust 1.89 or newer is required. From the workspace root, validate and build:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo build --workspace --release --all-features
cargo package -p wcsng-sigmf-derive -p wcsng-sigmf --all-features
```

Cargo verifies the packaged sources and writes the distributable `.crate`
archives to `target/package/`. For a reviewed, uncommitted release candidate,
add `--allow-dirty` to the packaging command; prefer a clean, committed worktree
for publication.

Publish in dependency order, waiting for each dependency to be available on
crates.io before publishing its dependent:

```sh
cargo publish -p wcsng-sigmf-derive
cargo publish -p wcsng-sigmf
```

These commands upload to crates.io and require authentication and ownership of
the crate names. Packaging alone does not publish anything.

### Publishing from GitHub Actions

In this repository's **Settings > Secrets and variables > Actions**, create a
repository secret named `CRATES_IO_TOKEN` containing a crates.io API token with
permission to publish both crates. The token's account will own newly published
crates; GitHub's repository account does not determine crates.io ownership.
Never commit the token.

The [publishing workflow](.github/workflows/publish.yml) runs when a `v*` tag is
pushed. Both public crates' manifest versions must match the tag (for example,
`v0.1.0` requires version `0.1.0` in both manifests). Update the workspace's
internal dependency versions when preparing a new release.

After committing and pushing the release changes:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The workflow validates all features, builds release libraries, saves the `.crate`
archives as an Actions artifact, and publishes the derive crate before the SigMF
crate. Cargo waits for the published dependency to become available in the
registry. The secret is mapped to `CARGO_REGISTRY_TOKEN` only for publishing.
Published versions cannot be overwritten; if a release partially succeeds, publish
the remaining crate separately rather than rerunning the completed publication.
