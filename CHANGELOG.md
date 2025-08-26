# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions workflow for automated builds
- Pre-compiled binaries for Windows, Linux, and macOS
- Comprehensive README with installation instructions
- MIT License

### Changed
- Improved project structure and documentation

## [1.0.0] - Initial Release

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

### Features
- Generate 1-32 markers per session
- Perceptually optimal color selection
- Brightness alternation for maximum contrast
- Professional output suitable for 3D scanning
- Cross-platform GUI built with egui/eframe
