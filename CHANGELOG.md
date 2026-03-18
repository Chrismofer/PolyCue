# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-18

### Added
- **Serial numbers** — optional numbering on each tag. Includes color picker, border toggle, and position sliders.
- **TTF font rendering** for serial numbers using `ab_glyph`.
- **Background color picker** — choose the background color for all tags. applies in preview and in saved files.
- **PNG output resolution control** — choose arbitrary resolution, defaults to 1600x1600
- **Configurable preview resolution** — defaulting to 300px tags for fast operation.
- **Windows icon embedding** — place `assets/icon.ico` in the project folder and it will be automatically embedded into the built `.exe` via `build.rs` using `winres`.
- **How to Use** section in the README — tables covering every control in the left panel, right panel, and grid area.
- macOS ARM64 (`aarch64-apple-darwin`) target added to the GitHub Actions CI/CD matrix.

### Changed
- **Center dot and gradient dot** sliders now go up to 100% of tag width (was capped at 50% before).
- **UI layout** reorganized into a left group (tag options) and right group (actions and display), with consistent padding and spacing throughout the toolbar. moved comuns slider from the top toolbar into the grid panel area.
- GitHub Actions: updated all deprecated action versions (`actions-rs/toolchain` → `dtolnay/rust-toolchain`, artifact/cache actions → v4, `softprops/action-gh-release` → v2).
- README: corrected Linux and macOS sections — no pre-built binaries exist yet for those platforms; build-from-source instructions added.

### Fixed
- **Preview resolution slider** was previously a dead control — now correctly wired into the preview render pipeline.
- Resolution values constrained to even numbers to prevent misshapen tags especially at ultra low resolutions.
- Build pipeline no longer fails on macOS and linux due to deprecated action versions or missing ARM64 target.

## [0.1.0] - Initial Release

### Added
- Interactive GUI for fiducial marker generation
- Support for 3-6 sided polygonal markers
- CIE Lab color space optimization with ΔE calculations
- Monte Carlo color grouping algorithm
- High-resolution PNG output (1600×1600px)
- JSON manifest with color metadata
- Real-time preview with customizable parameters
- Optional center dots and gradient effects
- Parallel processing for performance
- Async blur effects in preview panel
- Cross-platform GUI built with egui/eframe
- GitHub Actions workflow for automated Windows builds

