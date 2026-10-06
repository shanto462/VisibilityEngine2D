# Contributing

Thanks for your interest in improving VisibilityEngine2D. Bug reports, feature ideas, and pull requests are all welcome.

By taking part in this project you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). To report a security problem, follow [SECURITY.md](SECURITY.md) and do not open a public issue.

## Development setup

1. Install Rust with [rustup](https://rustup.rs/). The stable toolchain is enough. The app needs Rust 1.95 or newer and the `visibility-core` library needs 1.88 or newer (the `rust-version` fields in the `Cargo.toml` files).
2. Make sure you have the `rustfmt` and `clippy` components:

   ```sh
   rustup component add rustfmt clippy
   ```

3. Clone the repository and run the app:

   ```sh
   git clone https://github.com/shanto462/VisibilityEngine2D.git
   cd VisibilityEngine2D
   cargo run --release
   ```

The workspace has two crates:

- `crates/visibility-core`: the algorithms. No GUI dependencies.
- `crates/visibility-app`: the desktop app (egui and wgpu).

## Checks to run before you open a pull request

CI runs these exact commands. Please run them locally first:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

CI also runs these, which you can run if you have the tools installed:

- `cargo deny check` (dependency licenses, advisories, and sources; needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny))
- `cargo llvm-cov --workspace` (test coverage; needs [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov))
- A build and test run of each crate on its MSRV toolchain

If you change an algorithm, also run the benchmark and mention the numbers in the pull request:

```sh
cargo run --release -p visibility-core --example bench
```

## Tests

- Algorithm changes need a test in `crates/visibility-core/tests/correctness.rs`. The existing tests compare results with brute-force ray casting, which catches most mistakes.
- Keep new test data synthetic, like the random scenes and hand-made polygons the tests use today.

## Screenshots and local files

Images used by the README live in `assets/`. Rebuild them with `scripts/readme-images.sh` (it opens the app several times; keep the window in front). Put any other local output in `samples/output/`, which Git ignores.

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Start each commit message, and the pull request title, with a type:

- `feat:` a new feature, for example `feat: add polygon drawing tool`
- `fix:` a bug fix, for example `fix: keep the viewer inside the world bounds`
- `perf:` a performance improvement
- `refactor:` a code change with no behavior change
- `test:` tests only
- `docs:` documentation only
- `ci:` CI workflows
- `deps:` dependency updates
- `chore:` anything else

Add `!` after the type for a breaking change, for example `feat!: return visible ids as a bit set`.

## Pull request flow

1. Fork the repository and create a branch from `main`, for example `fix/viewer-bounds`.
2. Make your change. Add or update tests for it.
3. If users will notice the change, add a line to `CHANGELOG.md` under `## [Unreleased]`.
4. Run the checks above.
5. Open a pull request to `main` and fill in the template.
6. Wait for review. The maintainer may ask for changes.

## Branch rules

- `main` is protected. All changes go through a pull request.
- A pull request can merge only when the **`CI result`** status check is green. This one check passes only when every CI job passes (format, clippy, tests on Linux, macOS and Windows, MSRV, docs, cargo-deny, and coverage).
- Keep pull requests small and focused. One topic per pull request is easier to review.

## Releases

Releases are made by the maintainer. The maintainer bumps the version in the root `Cargo.toml`, moves the `## [Unreleased]` notes in `CHANGELOG.md` to a new `## [x.y.z]` section, and pushes a `vx.y.z` tag. The release workflow then builds the binaries, attests them, and publishes the GitHub Release.
