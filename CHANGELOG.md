# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-10-07

Complete rewrite in Rust. The C# WPF app is replaced by a Rust library
(`visibility-core`) and a desktop app (`visibility-engine-2d`) for Windows and
macOS.

### Added

- Exact, event-driven visibility polygons for a full circle or a view cone. The
  outline follows obstacle edges exactly, including where obstacles overlap.
- Exact occlusion culling: an obstacle is visible when any part of it can be seen.
- Exact frustum culling with a trigonometry-free sector test.
- GPU rendering with wgpu: obstacles are uploaded once, so panning and zooming stay
  smooth with 100,000 obstacles.
- Light and dark themes, a side panel with live timings and algorithm counters,
  rays toward obstacle corners in shadow casting and occlusion modes, live viewer
  dragging, and keyboard shortcuts.
- Adjustable range, number of obstacles (up to 200,000), world size and random seed.
- Command-line options for every setting, plus `--screenshot` for scripted images.
- Release binaries for Windows (x64, ARM64), macOS (Apple Silicon, Intel) and Linux
  (x64) with SHA-256 checksums and build provenance attestations.
- CI on Linux, macOS and Windows, MSRV checks, CodeQL, OpenSSF Scorecard,
  cargo-deny and Dependabot.

### Fixed

- Shadow casting no longer cuts corners or misses thin obstacles between its fixed
  rays, and no longer has a data race from adding results on parallel threads.
- Frustum culling no longer misses obstacles that cross only the curved arc of the
  cone.
- Occlusion culling no longer reports obstacles as hidden when only their corners
  are hidden, and obstacles behind a target no longer hide it.

### Removed

- The C# WPF app and its Visual Studio solution.

[Unreleased]: https://github.com/shanto462/VisibilityEngine2D/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/shanto462/VisibilityEngine2D/releases/tag/v1.0.0
