## Summary

<!-- What does this pull request change, and why? Link the issue if there is one, for example "Closes #12". -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Performance improvement
- [ ] Refactor (no behavior change)
- [ ] Documentation
- [ ] CI, build, or dependencies

## How I tested it

<!-- For example: "cargo test --workspace --locked", plus a manual run like "cargo run --release -- --mode occlusion --rays". For algorithm changes, paste the benchmark output before and after. -->

## Checklist

- [ ] The PR title follows [Conventional Commits](https://www.conventionalcommits.org/) (for example `feat: add polygon drawing tool`).
- [ ] `cargo fmt --all --check` passes.
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- [ ] `cargo test --workspace --locked` passes.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` passes.
- [ ] I added or updated tests for the change.
- [ ] I updated `CHANGELOG.md` under `## [Unreleased]` if users will notice the change.
