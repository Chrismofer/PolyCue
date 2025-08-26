# Poly Cue Tag Generator

A Rust application for generating polygonal fiducial markers optimized for photogrammetry and computer vision applications.



<img width="1006" height="648" alt="image" src="https://github.com/user-attachments/assets/fa87b147-154d-4413-82b0-b66cfa40f483" />



## Features

- **Advanced Color Selection**: Uses CIE Lab color space and ΔE calculations to ensure maximum visual distinction between colors and calculates the maximum number of tags possible based on available distinct colors
- **Optimized Color Grouping**: Monte Carlo optimization algorithm arranges colors for optimal contrast between adjacent segments<img width="1843" height="1152" alt="image" src="https://github.com/user-attachments/assets/fdc40e75-ae7f-4ca2-8a35-add819f21eeb" />

- **High-Resolution Output**
- **Multiple Polygon Types**: Support for 3-6 sided markers (triangular, square, pentagonal, hexagonal)
- **Customizable Features**: Optional center dot and gradient dot with adjustable sizes
- **Timestamped Output**: Automatically organizes generated files in dated subdirectories
- **Performance Optimized**: Parallel processing, async rendering, and efficient regeneration for smooth interaction
- **Responsive Interface**: Real-time regeneration when resizing panels for optimal viewing

## Quick Start

### Option 1: Download Pre-built Binary (Recommended)

**Windows:**
1. Go to [Releases](https://github.com/Chrismofer/PolyCue/releases)
2. Download `polycue-windows-x64.zip`
3. Extract and run `polycue-windows-x64.exe`

**Linux:**
1. Download `polycue-linux-x64.tar.gz`
2. Extract: `tar -xzf polycue-linux-x64.tar.gz`
3. Run: `./polycue-linux-x64`

**macOS:**
1. Download `polycue-macos-x64.tar.gz`
2. Extract: `tar -xzf polycue-macos-x64.tar.gz`
3. Run: `./polycue-macos-x64`

### Option 2: Build from Source

**Prerequisites:** [Rust](https://rustup.rs/) (latest stable version)

```bash
# Clone the repository
git clone https://github.com/Chrismofer/PolyCue.git
cd PolyCue

# Run the application
cargo run --release
```

**Windows users can also use the build script:**
```batch
# Double-click build-windows.bat or run in Command Prompt
build-windows.bat
```

The GUI will open, allowing you to:
1. Adjust the number of markers (dynamically limited based on available colors)
2. Change polygon sides (3-6)
3. Toggle center/gradient dots with size controls
4. Adjust preview resolution and grid layout
5. Use the Regenerate button to manually refresh
6. Save markers separately or together in a combined grid image
7. Enable profiling logs for performance monitoring
8. Toggle defer high-res for better interactive performance
9. Resize panels for optimal viewing (auto-regenerates previews)

### Output

Generated files are saved to timestamped subdirectories in the `output/` directory:
- `output/2025-08-24_14-30-45/` - Timestamped folder for each generation session
- `tag_01.png`, `tag_02.png`, etc. - High-resolution marker images (1600×1600px) when using "Save All Separate"
- `all_tags_combined.png` - Single grid image containing all tags when using "Save All Together"
- `manifest.json` - Metadata including RGB values, Lab coordinates, and color separation metrics

## How It Works

### Color Selection Algorithm

1. **Candidate Generation**: Creates a grid of 216 perceptually-spaced sRGB colors
2. **Lightness Filtering**: Removes colors that are too dark (L* < 20) or too bright (L* > 90)
3. **Threshold Optimization**: Binary search to find the maximum ΔE threshold that provides enough distinct colors
4. **Greedy Selection**: Picks colors that meet the minimum separation requirement
5. **Dynamic Limits**: Automatically calculates and displays the maximum possible tags for current settings

### Color Arrangement

1. **Monte Carlo Grouping**: Uses 2000 iterations to optimally assign colors to marker groups
2. **Brightness Alternation**: For even-sided polygons, alternates bright and dark colors for maximum adjacent contrast
3. **Validation**: Ensures minimum pairwise ΔE within each marker meets quality standards

### Technical Details

- **Color Science**: CIE76 ΔE calculations for perceptually uniform color differences
- **Rendering**: Custom triangle rasterization with anti-aliasing support
- **Performance**: Parallel processing using Rayon, async blur effects, smart regeneration
- **GUI Framework**: Built with egui/eframe for cross-platform compatibility

## Interface Features

The application features a split-panel interface:

**Left Panel (Main Grid):**
- Primary tag preview grid with configurable columns
- Resizable panel that automatically updates previews when adjusted
- Real-time preview at adjustable resolution

**Right Panel (Post-Processing Preview):**
- Monochrome half-size versions of all tags
- First tag at multiple scaled sizes (0.5x to 0.01x)
- Gaussian blur effects with animated loading placeholders

**Top Control Bar:**
- Count slider with dynamic maximum based on available colors
- Polygon sides selector (3-6 sides)
- ΔE threshold display (automatically calculated)
- Regenerate button for manual refresh
- Save All Separate button (saves individual PNG files)
- Save All Together button (saves combined grid image)
- Center dot and gradient dot controls with size adjustment
- Resolution and layout controls
- Profiling logs checkbox (enables performance timing output)
- Defer high-res checkbox (performance optimization option)

## Configuration

Key parameters can be adjusted in the GUI:

| Parameter | Range | Description |
|-----------|-------|-------------|
| Count | 1-Dynamic Max | Number of markers (max calculated automatically) |
| Sides | 3-6 | Polygon sides per marker |
| Center Dot | Toggle + Size | Optional identification dot (1-50% size) |
| Gradient Dot | Toggle + Size | Optional gradient effect (1-50% size) |
| Resolution | 2-2000px | Preview resolution (save is always 1600×1600) |
| Columns | 1-8 | Grid layout for preview display |
| Profiling Logs | Checkbox | Enables detailed performance timing output to console |
| Defer High-res | Checkbox | Skip high-res rendering during interactive changes for better performance |

**Dynamic Limits Example:**
- 3-sided polygons: Up to ~72 tags possible
- 4-sided polygons: Up to ~54 tags possible  
- 5-sided polygons: Up to ~43 tags possible
- 6-sided polygons: Up to ~36 tags possible

*(Actual limits depend on color distinctness requirements)*

## Use Cases

- **3D Scanning**: Fiducial markers for photogrammetry and structured light scanning
- **Robotics**: Visual landmarks for SLAM and navigation
- **Augmented Reality**: Tracking markers for AR applications
- **Computer Vision Research**: Standardized test patterns

## Technical Architecture

```
Color Pool Generation → ΔE Optimization → Monte Carlo Grouping → Rendering Pipeline
        ↓                      ↓                    ↓                    ↓
   216 candidates → Binary search threshold → Optimal assignments → Timestamped output
```




## Theory behind the markers (Robust Multicolor Fiducials for Structure-From-Motion Pipelines):

## Purpose of markers
Structure-From-Motion (SfM) pipelines use distinctive local features to compute camera poses, and are challenged by featureless or repeating-detail surfaces. Fiducial markers (tags) can be placed on or around an object to provide high-contrast anchor points. 

## Problem space, Deformations, and Robustness
In practice these tags are affixed to surfaces that are moving and rotating, and are photographed from multiple angles, causing variable distortion and occlusion to the tags. In addition to geometric distortions, cameras may operate with a wide aperture to enable low ISO and high shutter speed (to minimize digital noise as well as motion blur) which results in a relatively shallow depth-of-field. Even with a lot of light, areas in the foreground and background will be slightly defocused, including tags located there. This also challenges the consistency and reliability of feature matching. A robust tag design should remain both visible and uniquely identifiable despite these deformations as well as those imposed by the feature detection algorithms.

## April tags limitations
April tags, one commonly used black-and-white fiducial tag, are technically visually distinct from one another, but SfM’s general-purpose detectors simply can’t leverage that ID. a feature detector such as SURF or SIFT blurs the image to look for features like blobs, edges, and corners. After the regular visual deformations and this Gaussian blurring step, April tags below a certain size are not easily visually separable. Most dense black and white tags (ARTag, April tag, QR code, Stag, Maxicode, etc) devolve into a gray smudge when blurred, and lose all distinctness. 

## SIFTtag (smooth gradient dot)
SfM pipelines rely on a variety of feature detection and tracking algorithms, sometimes proprietary ones, to detect and track features across frames. Classical (SIFT, SURF, ORB, FAST, MSER, BRISK, or CNN) approaches locate features that are invariant to scale, rotation, and illumination. Techniques like SIFT apply multi-scale Gaussian blurring to the image and identify extrema in the Difference-of-Gaussians (DoG) space, to isolate distinct blobs that remain stable across transformations.

An optimal tag for SIFT feature detection has been developed and looks like a smooth gradient dot, characterized by its lack of high-frequency edge content: https://www.researchgate.net/publication/220839283_Maximum_Detector_Response_Markers_for_SIFT_and_SURF

Unlike April tags, ARTtags, and others, SIFTtag exhibits minimal deformation under transformation. A smooth gradient dot has no high frequency edges, it remains almost unaffected by blurring, and is largely rotation, scale, and deformation invariant. 


## SIFTtag robustness vs. precision
While radial isotropy and a lack of sharp edges makes this tag maximally robust, it also makes it minimally precise for pose estimation (due to the flat gradient slope in the center of the dot and at the edges). It is desirable to combine the smooth gradient dots with some sort of high contrast sharp-edged features, so that the detector has enough information to robustly and also precisely estimate the camera poses. 

## Entropy vs. Robustness
Fine features such as patterns of dots or lines provide a better ‘descriptor’ but vanish entirely under defocus or motion. Large blocks of alternating brightness provide sharp edges that also persist through blurring. By surrounding a smooth gradient dot with these blocks, we could make one composite tag which contains both low-frequency robustness and high-frequency precision.


## Self-Similarity
Self-similar tags, especially when arranged in a repeating pattern, increase the risk of false positive matches in SfM pipelines. Identical tags produce nearly indistinguishable descriptors (e.g. SIFT, ORB), making it hard to tell which tag is which across frames. Visually separable tags reduce ambiguity. The regularity of the pattern-of-tags is made irrelevant if the tags themselves are visually distinct, as the matching algorithm can then more easily reject false positives.


## ‘Poly Cues’ Polychromatic Polygons
Conventionally, fiducial tags are black and white to maximize contrast. Since we use a color camera, one easy way to make otherwise identical tags visually unique is to simply make them different colors. Polycues are generated using a set of unique colors picked from a colorspace to maximize ΔE (perceptual color difference metric). These tags consist of a regular polygon, divided into slices radially. Each slice is a different color. In the center is a smooth Gaussian dot. This design contains the benefits of a SIFTtag, plus rotational asymmetry, tag to tag differentiability, and sharp edges and junctions for precision pose estimation.

<img width="1009" height="965" alt="image" src="https://github.com/user-attachments/assets/bd1c9836-684d-46b1-a9c2-52b92cbf4b1f" />


## Number of sides
The choice of sides has two confounding constraints. A smaller number of divisions means each is larger, and will more easily survive the DoG and camera defocus.
But also, a small number of sides, 3 for instance, causes the vertices to be very pointy and therefore extra susceptible to defocus, motion blur, and DoG. 4-gons have square corners. Corners, edges, and ‘blobs’ are all detected as trackable features by SfM software. As you add sides the corners get closer and closer to 180 degrees, and the N-gon approximates a circle, lacking trackable corners. The program can only generate so many unique tags based on its method of choosing unique colors from a color space to maximize their difference. It can generate up to 62 3-sided tags, 46 4-sided tags, 37 5-sided tags, 31 6-sided tags, etc. 

Seemingly 4 sides are best to balance all constraints, unless a small number of very large tags are to be used, in which case more than 4 sides may be beneficial. 





## Performance

- **Color Selection**: ~50ms for complex selections
- **Rendering**: ~100ms for high-resolution images (parallel processing)
- **GUI Updates**: <16ms for smooth 60fps interaction
- **Memory Usage**: ~50MB typical working set
- **Smart Regeneration**: Only updates what's needed when UI changes
- **Async Processing**: Non-blocking blur effects and preview generation

## Recent Improvements

- **Dynamic Slider Limits**: Count slider maximum adjusts based on available distinct colors
- **Timestamped Output**: Files organized in dated subdirectories
- **Responsive Panels**: Automatic regeneration when resizing interface panels  
- **Enhanced Previews**: Multiple scaled versions and blur effects in right panel
- **Performance Optimizations**: Debounced regeneration and smart caching

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## Acknowledgments

- Color science algorithms based on CIE standards
- Built with the excellent [egui](https://github.com/emilk/egui) immediate mode GUI framework
- Parallel processing powered by [Rayon](https://github.com/rayon-rs/rayon)

---
