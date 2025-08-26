use serde::Serialize;
use image::{DynamicImage, Rgb};
use palette::Lab;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Local};
use crate::color::{srgb_u8_to_lab, delta_e};

#[derive(Debug, Serialize)]
pub struct TagManifestEntry {
    pub filename: String,
    pub sides: usize,
    pub colors_rgb: Vec<(u8, u8, u8)>,
    pub colors_lab: Vec<(f32, f32, f32)>,
    pub min_pairwise_delta_e: f32,
}

#[derive(Serialize)]
struct Manifest {
    threshold: f32,
    tags: Vec<TagManifestEntry>,
}

/// Ensure output directory exists
pub fn ensure_out_dir(path: &str) -> std::io::Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Save all generated tags and manifest to disk
pub fn save_all(
    tags: &[Vec<Rgb<u8>>], 
    threshold: f32, 
    images: &[DynamicImage], 
    sides: usize
) -> Result<(), Box<dyn std::error::Error>> {
    // Create timestamped subdirectory
    let now: DateTime<Local> = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let out_dir = format!("output/{}", timestamp);
    ensure_out_dir(&out_dir)?;

    let mut manifest = Manifest { threshold, tags: Vec::new() };
    
    for (idx, colors) in tags.iter().enumerate() {
        let filename = format!("tag_{:02}.png", idx + 1);
        let path = format!("{}/{}", out_dir, &filename);
        
        // Save from the high-resolution buffer
        if let Some(img) = images.get(idx) {
            img.save(&path)?;
        }

        let labs_vec: Vec<Lab> = colors.iter().copied().map(srgb_u8_to_lab).collect();
        
        // Compute min pairwise ΔE
        let mut min_pair = f32::INFINITY;
        for i in 0..labs_vec.len() {
            for j in (i + 1)..labs_vec.len() {
                let d = delta_e(labs_vec[i], labs_vec[j]);
                if d < min_pair { min_pair = d; }
            }
        }

        manifest.tags.push(TagManifestEntry {
            filename,
            sides,
            colors_rgb: colors.iter().map(|c| (c[0], c[1], c[2])).collect(),
            colors_lab: labs_vec.iter().map(|l| (l.l, l.a, l.b)).collect(),
            min_pairwise_delta_e: min_pair,
        });
    }

    let mut file = File::create(format!("{}/manifest.json", out_dir))?;
    let json = serde_json::to_string_pretty(&manifest)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Save all tags combined into a single grid image
pub fn save_all_together(
    tags: &[Vec<Rgb<u8>>], 
    threshold: f32, 
    images: &[DynamicImage], 
    sides: usize
) -> Result<(), Box<dyn std::error::Error>> {
    if images.is_empty() {
        return Ok(());
    }
    
    // Create timestamped subdirectory
    let now: DateTime<Local> = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let out_dir = format!("output/{}", timestamp);
    ensure_out_dir(&out_dir)?;

    // Calculate grid dimensions (try to make it roughly square)
    let count = images.len();
    let cols = (count as f32).sqrt().ceil() as usize;
    let rows = (count + cols - 1) / cols; // Ceiling division
    
    // Get individual image size (assuming all are same size)
    let img_width = images[0].width();
    let img_height = images[0].height();
    
    // Create combined image
    let combined_width = cols as u32 * img_width;
    let combined_height = rows as u32 * img_height;
    let mut combined = image::ImageBuffer::new(combined_width, combined_height);
    
    // Fill with white background
    for pixel in combined.pixels_mut() {
        *pixel = image::Rgb([255, 255, 255]);
    }
    
    // Place each tag image in the grid
    for (idx, img) in images.iter().enumerate() {
        let col = idx % cols;
        let row = idx / cols;
        let x_offset = col as u32 * img_width;
        let y_offset = row as u32 * img_height;
        
        let rgb_img = img.to_rgb8();
        for (x, y, pixel) in rgb_img.enumerate_pixels() {
            if x_offset + x < combined_width && y_offset + y < combined_height {
                combined.put_pixel(x_offset + x, y_offset + y, *pixel);
            }
        }
    }
    
    // Save combined image
    let combined_path = format!("{}/all_tags_combined.png", out_dir);
    image::DynamicImage::ImageRgb8(combined).save(&combined_path)?;
    
    // Also save manifest
    let mut manifest = Manifest { threshold, tags: Vec::new() };
    
    for (idx, colors) in tags.iter().enumerate() {
        let filename = format!("tag_{:02}_in_combined.png", idx + 1);
        let labs_vec: Vec<Lab> = colors.iter().copied().map(srgb_u8_to_lab).collect();
        
        // Compute min pairwise ΔE
        let mut min_pair = f32::INFINITY;
        for i in 0..labs_vec.len() {
            for j in (i + 1)..labs_vec.len() {
                let d = delta_e(labs_vec[i], labs_vec[j]);
                if d < min_pair { min_pair = d; }
            }
        }

        manifest.tags.push(TagManifestEntry {
            filename,
            sides,
            colors_rgb: colors.iter().map(|c| (c[0], c[1], c[2])).collect(),
            colors_lab: labs_vec.iter().map(|l| (l.l, l.a, l.b)).collect(),
            min_pairwise_delta_e: min_pair,
        });
    }

    let mut file = File::create(format!("{}/manifest.json", out_dir))?;
    let json = serde_json::to_string_pretty(&manifest)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}
