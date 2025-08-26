use eframe::egui::{self, ColorImage, Context, TextureHandle, TextureOptions};
use image::{DynamicImage, Rgb};
use image::imageops::FilterType;
use palette::Lab;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::thread;
use rayon::prelude::*;

use crate::color::{candidate_srgb_grid, srgb_u8_to_lab, compute_max_threshold_and_colors_from_pool, reorder_bright_dark_alternating};
use crate::render::{group_colors_into_groups_monte_carlo, draw_marker_polygon};
use crate::io::{save_all, save_all_together};

// ============================================================================
// SLIDER CONFIGURATION - Easily adjust all UI control ranges and defaults here
// ============================================================================

pub struct SliderConfig;

impl SliderConfig {
    // Tag Count Slider
    pub const COUNT_MIN: i32 = 1;
    pub const COUNT_MAX: i32 = 100;
    pub const COUNT_DEFAULT: usize = 8;
    
    // Polygon Sides Slider
    pub const SIDES_MIN: i32 = 3;
    pub const SIDES_MAX: i32 = 6;
    pub const SIDES_DEFAULT: usize = 4;
    
    // Center Dot Size Slider (percentage)
    pub const CENTER_DOT_MIN: f32 = 1.0;
    pub const CENTER_DOT_MAX: f32 = 50.0;
    pub const CENTER_DOT_STEP: f64 = 1.0;
    pub const CENTER_DOT_DEFAULT: f32 = 35.0;
    
    // Gradient Dot Size Slider (percentage)
    pub const GRADIENT_DOT_MIN: f32 = 1.0;
    pub const GRADIENT_DOT_MAX: f32 = 50.0;
    pub const GRADIENT_DOT_STEP: f64 = 1.0;
    pub const GRADIENT_DOT_DEFAULT: f32 = 35.0;
    
    // Tag Resolution Slider
    pub const RESOLUTION_MIN: f32 = 2.0;
    pub const RESOLUTION_MAX: f32 = 2000.0;
    pub const RESOLUTION_DEFAULT: u32 = 1000;
    
    // Grid Columns Slider
    pub const COLUMNS_MIN: i32 = 1;
    pub const COLUMNS_MAX: i32 = 8;
    pub const COLUMNS_DEFAULT: usize = 4;
    
    // Other Default Values
    pub const THRESHOLD_DEFAULT: f32 = 28.0;
    pub const SAVE_SIZE_DEFAULT: (u32, u32) = (1600, 1600);
    pub const TILE_WIDTH_DEFAULT: f32 = 256.0;
    pub const CENTER_DOT_ENABLED_DEFAULT: bool = true;
    pub const GRADIENT_DOT_ENABLED_DEFAULT: bool = true;
    pub const PROFILING_DEFAULT: bool = true;
    pub const DEFER_HIGH_RES_DEFAULT: bool = true;
}

// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegenKind {
    Full,
    ImagesOnly,
}

pub struct AppState {
    pub count: usize,
    pub threshold: f32,
    pub sides: usize,
    pub tags: Vec<Vec<Rgb<u8>>>,
    pub textures: Vec<TextureHandle>,
    pub save_size: (u32, u32),
    pub high_res: Vec<DynamicImage>,
    pub preview_max_width: u32,
    pub columns: usize,
    pub center_dot: bool,
    pub center_dot_size_pct: f32,
    pub gradient_dot: bool,
    pub gradient_dot_size_pct: f32,
    
    // Maximum possible count based on available colors
    pub max_possible_count: usize,
    
    // Debounced regeneration
    pub pending_regen: Option<RegenKind>,
    pub regen_deadline: Option<Instant>,
    
    // Cached candidate pool to speed up full regenerations
    pub candidate_pool: Vec<Rgb<u8>>,
    pub candidate_labs: Vec<Lab>,
    
    // Right panel preview caches
    pub right_mono_textures: Vec<TextureHandle>,
    pub right_first_scaled_textures: Vec<TextureHandle>,
    pub right_blurred_textures: Vec<Option<TextureHandle>>,
    
    // Tracks current tile width of left grid (for right-panel sizing)
    pub last_left_tile_w: f32,
    
    // Track panel width for resize detection
    pub last_panel_width: f32,
    
    // Verbose timing logs toggle
    pub profiling: bool,
    
    // If true, skip high-res render on interactive changes; only render on Save
    pub defer_high_res: bool,
    
    // Async blur job
    pub blur_job_id: u64,
    pub blurred_rx: Option<mpsc::Receiver<(u64, usize, image::RgbaImage)>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut app = AppState {
            count: SliderConfig::COUNT_DEFAULT,
            threshold: SliderConfig::THRESHOLD_DEFAULT,
            sides: SliderConfig::SIDES_DEFAULT,
            tags: Vec::new(),
            textures: Vec::new(),
            save_size: SliderConfig::SAVE_SIZE_DEFAULT,
            high_res: Vec::new(),
            preview_max_width: SliderConfig::RESOLUTION_DEFAULT,
            columns: SliderConfig::COLUMNS_DEFAULT,
            center_dot: SliderConfig::CENTER_DOT_ENABLED_DEFAULT,
            center_dot_size_pct: SliderConfig::CENTER_DOT_DEFAULT,
            gradient_dot: SliderConfig::GRADIENT_DOT_ENABLED_DEFAULT,
            gradient_dot_size_pct: SliderConfig::GRADIENT_DOT_DEFAULT,
            max_possible_count: SliderConfig::COUNT_MAX as usize,
            pending_regen: None,
            regen_deadline: None,
            candidate_pool: Vec::new(),
            candidate_labs: Vec::new(),
            right_mono_textures: Vec::new(),
            right_first_scaled_textures: Vec::new(),
            right_blurred_textures: Vec::new(),
            last_left_tile_w: SliderConfig::TILE_WIDTH_DEFAULT,
            last_panel_width: 800.0, // default width
            profiling: SliderConfig::PROFILING_DEFAULT,
            defer_high_res: SliderConfig::DEFER_HIGH_RES_DEFAULT,
            blur_job_id: 0,
            blurred_rx: None,
        };
        
        // Build cached candidate pool once
        let mut pool = candidate_srgb_grid();
        // Filter by lightness range using Lab
        pool.retain(|&c| {
            let l = srgb_u8_to_lab(c).l;
            (20.0..=90.0).contains(&l)
        });
        let labs = pool.iter().copied().map(srgb_u8_to_lab).collect();
        app.candidate_pool = pool;
        app.candidate_labs = labs;
        
        // Calculate initial max possible count
        app.update_max_possible_count();
        
        app
    }

    pub fn update_max_possible_count(&mut self) {
        // Estimate max possible tags by attempting to find colors for a large number
        // and seeing how many we can actually get
        let test_needed = 1000 * self.sides; // test with a very high number
        let (_threshold, colors) = compute_max_threshold_and_colors_from_pool(
            &self.candidate_pool, 
            &self.candidate_labs, 
            test_needed
        );
        self.max_possible_count = (colors.len() / self.sides).max(1);
    }

    pub fn schedule_regen(&mut self, kind: RegenKind, delay_ms: u64) {
        // If a full regen is requested, it overrides images-only
        match (self.pending_regen, kind) {
            (Some(RegenKind::Full), _) => {
                // already have full scheduled; keep earliest deadline
            }
            (_, RegenKind::Full) => {
                self.pending_regen = Some(RegenKind::Full);
            }
            (None, RegenKind::ImagesOnly) => {
                self.pending_regen = Some(RegenKind::ImagesOnly);
            }
            (Some(RegenKind::ImagesOnly), RegenKind::ImagesOnly) => {
                // keep as images-only
            }
        }
        let new_deadline = Instant::now() + Duration::from_millis(delay_ms);
        self.regen_deadline = Some(match self.regen_deadline {
            Some(old) => old.min(new_deadline),
            None => new_deadline,
        });
    }

    pub fn regenerate(&mut self, ctx: &Context) {
        let t_total = Instant::now();
        if self.profiling { println!("[profile] regenerate: start"); }
        
        // Ensure sides stays within [3, 6]
        self.sides = self.sides.clamp(3, 6);
        
        // Auto-compute max feasible ΔE for the requested number of tags
        let needed = self.count.saturating_mul(self.sides).max(self.sides);
        
        // Use cached candidate pool for speed
        let t0 = Instant::now();
        let (auto_thr, mut colors) = compute_max_threshold_and_colors_from_pool(&self.candidate_pool, &self.candidate_labs, needed);
        if self.profiling { println!("[profile] \tcolor select: {:.2} ms (needed={})", t0.elapsed().as_secs_f64()*1000.0, needed); }
        
        self.threshold = auto_thr;
        if colors.len() < needed {
            // If not enough colors, reduce count to what's possible
            self.count = (colors.len() / self.sides).max(1);
            colors.truncate(self.count * self.sides);
        }
        
        let labs: Vec<Lab> = colors.iter().copied().map(srgb_u8_to_lab).collect();
        let t1 = Instant::now();
        self.tags = group_colors_into_groups_monte_carlo(colors, labs, self.count, self.sides, 2000);
        if self.profiling { println!("[profile] \tgrouping: {:.2} ms (tags={}, sides={})", t1.elapsed().as_secs_f64()*1000.0, self.count, self.sides); }
        
        // For even-sided markers, reorder each tag to alternate bright/dark to maximize adjacent contrast
        if self.sides % 2 == 0 {
            let t2 = Instant::now();
            for tag in &mut self.tags { 
                reorder_bright_dark_alternating(tag); 
            }
            if self.profiling { println!("[profile] \treorder: {:.2} ms", t2.elapsed().as_secs_f64()*1000.0); }
        }
        
        self.textures.clear();
        self.high_res.clear();

        // Render high-resolution images once
        if !self.defer_high_res {
            let t3 = Instant::now();
            self.render_high_res_images();
            if self.profiling { println!("[profile] \trender_high_res: {:.2} ms", t3.elapsed().as_secs_f64()*1000.0); }
        }

        // Build lightweight previews (skip heavy high-res resize path)
        let t4 = Instant::now();
        self.rebuild_textures_quick(ctx);
        if self.profiling { println!("[profile] \tbuild_previews_quick: {:.2} ms", t4.elapsed().as_secs_f64()*1000.0); }
        if self.profiling { println!("[profile] regenerate: total {:.2} ms", t_total.elapsed().as_secs_f64()*1000.0); }
    }

    pub fn render_high_res_images(&mut self) {
        let t0 = Instant::now();
        self.high_res.clear();
        let sides = self.sides;
        let center_dot = self.center_dot;
        let center_dot_size_pct = self.center_dot_size_pct;
        let gradient_dot = self.gradient_dot;
        let gradient_dot_size_pct = self.gradient_dot_size_pct;
        let (w, h) = self.save_size;
        
        self.high_res = self
            .tags
            .par_iter()
            .map(|colors| {
                let img = draw_marker_polygon(
                    w,
                    h,
                    sides,
                    colors,
                    center_dot,
                    center_dot_size_pct,
                    gradient_dot,
                    gradient_dot_size_pct,
                );
                DynamicImage::ImageRgb8(img)
            })
            .collect();
        if self.profiling { println!("[profile] render_high_res_images: {:.2} ms (count={}, size={}x{})", t0.elapsed().as_secs_f64()*1000.0, self.tags.len(), self.save_size.0, self.save_size.1); }
    }

    pub fn rebuild_textures_quick(&mut self, ctx: &Context) {
        // Draw small square previews directly at left tile size
        let t0 = Instant::now();
        self.textures.clear();
        let w = self.last_left_tile_w.round().max(2.0) as u32;
        let h = w; // square preview
        let sides = self.sides;
        let center_dot = self.center_dot;
        let center_dot_size_pct = self.center_dot_size_pct;
        let gradient_dot = self.gradient_dot;
        let gradient_dot_size_pct = self.gradient_dot_size_pct;
        
        let imgs: Vec<_> = self
            .tags
            .par_iter()
            .enumerate()
            .map(|(i, colors)| {
                let img = draw_marker_polygon(w, h, sides, colors, center_dot, center_dot_size_pct, gradient_dot, gradient_dot_size_pct);
                (i, DynamicImage::ImageRgb8(img).to_rgba8())
            })
            .collect();
            
        for (i, rgba) in imgs.into_iter() {
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
            let tex = ctx.load_texture(format!("tag_preview_quick_{}", i), color_image, TextureOptions::NEAREST);
            self.textures.push(tex);
        }
        
        // Also refresh right-panel previews
        self.rebuild_right_textures_quick(ctx);
        if self.profiling { println!("[profile] rebuild_textures_quick: {:.2} ms (left previews={}, tile={}x{})", t0.elapsed().as_secs_f64()*1000.0, self.textures.len(), w, h); }
    }

    pub fn rebuild_right_textures_quick(&mut self, ctx: &Context) {
        // Half-size monochrome for all tags, scaled variants for first tag, and blurred versions
        self.right_mono_textures.clear();
        self.right_first_scaled_textures.clear();
        self.right_blurred_textures.clear();

        if self.tags.is_empty() {
            return;
        }

        // Use left tile width to size right-panel previews; cheaper and visually consistent
        let base_w = self.last_left_tile_w.round().max(2.0) as u32;
        let half_w = (base_w / 2).max(2);
        let half_h = half_w;
        
        // Monochrome half-size for all tags
        let t_mono = Instant::now();
        let sides = self.sides;
        let center_dot = self.center_dot;
        let center_dot_size_pct = self.center_dot_size_pct;
        let gradient_dot = self.gradient_dot;
        let gradient_dot_size_pct = self.gradient_dot_size_pct;
        
        let mono_rgba: Vec<_> = self
            .tags
            .par_iter()
            .enumerate()
            .map(|(i, colors)| {
                let rgb = draw_marker_polygon(half_w, half_h, sides, colors, center_dot, center_dot_size_pct, gradient_dot, gradient_dot_size_pct);
                (i, DynamicImage::ImageRgb8(rgb).grayscale().to_rgba8())
            })
            .collect();
            
        for (i, rgba) in mono_rgba.into_iter() {
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
            let tex = ctx.load_texture(format!("right_mono_{}", i), color_image, TextureOptions::NEAREST);
            self.right_mono_textures.push(tex);
        }
        if self.profiling { println!("[profile] \tright mono: {:.2} ms (count={}, size={}x{})", t_mono.elapsed().as_secs_f64()*1000.0, self.right_mono_textures.len(), half_w, half_h); }

        // First tag at multiple scales
        let first_colors = &self.tags[0];
        let scales: [f32; 18] = [
            0.5, 0.4, 0.3, 0.2, 0.15, 0.14, 0.13, 0.12, 0.1,
            0.09, 0.08, 0.07, 0.06, 0.05, 0.04, 0.03, 0.02, 0.01,
        ];
        let t_scaled = Instant::now();
        for (k, s) in scales.iter().enumerate() {
            let w = ((base_w as f32) * s).round().max(2.0) as u32;
            let h = w;
            let img = draw_marker_polygon(w, h, self.sides, first_colors, self.center_dot, self.center_dot_size_pct, self.gradient_dot, self.gradient_dot_size_pct);
            let rgba = DynamicImage::ImageRgb8(img).to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
            let tex = ctx.load_texture(format!("right_first_scaled_{}", k), color_image, TextureOptions::NEAREST);
            self.right_first_scaled_textures.push(tex);
        }
        if self.profiling { println!("[profile] \tright scaled: {:.2} ms (variants={}, base_w={})", t_scaled.elapsed().as_secs_f64()*1000.0, self.right_first_scaled_textures.len(), base_w); }

        // Gaussian blur: render and blur at a smaller working size, then upscale to display size
        let blur_dst_w = base_w.max(2);
        let blur_src_w: u32 = blur_dst_w.clamp(16, 128); // cap work size for speed
        let blur_src_h = blur_src_w;
        let base_small = draw_marker_polygon(blur_src_w, blur_src_h, self.sides, first_colors, self.center_dot, self.center_dot_size_pct, self.gradient_dot, self.gradient_dot_size_pct);
        let base_small_dyn = DynamicImage::ImageRgb8(base_small);
        let blur_levels: [f32; 6] = [0.03, 0.06, 0.10, 0.16, 0.22, 0.30];
        
        // Prepare placeholders so UI can show blanks immediately
        self.right_blurred_textures = vec![None; blur_levels.len()];
        
        // Spawn async blur job to compute each level and stream results
        self.blur_job_id = self.blur_job_id.wrapping_add(1);
        let job_id = self.blur_job_id;
        let (tx, rx) = mpsc::channel::<(u64, usize, image::RgbaImage)>();
        self.blurred_rx = Some(rx);
        let base_small_dyn_cloned = base_small_dyn.clone();
        
        thread::spawn(move || {
            for (i, k) in blur_levels.iter().enumerate() {
                let sigma_full = (blur_dst_w as f32 * k).clamp(0.5, 300.0);
                let scale = blur_src_w as f32 / blur_dst_w as f32;
                let sigma_small = (sigma_full * scale).max(0.5);
                let b_small = image::imageops::blur(&base_small_dyn_cloned, sigma_small);
                let b_up: DynamicImage = DynamicImage::ImageRgba8(b_small).resize_exact(blur_dst_w, blur_dst_w, FilterType::Triangle);
                let rgba = b_up.to_rgba8();
                let _ = tx.send((job_id, i, rgba));
            }
        });
    }

    pub fn save_current_tags(&mut self) {
        self.render_high_res_images();
        if let Err(e) = save_all(&self.tags, self.threshold, &self.high_res, self.sides) {
            eprintln!("Save failed: {}", e);
        }
    }

    pub fn save_current_tags_together(&mut self) {
        self.render_high_res_images();
        if let Err(e) = save_all_together(&self.tags, self.threshold, &self.high_res, self.sides) {
            eprintln!("Save together failed: {}", e);
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Keep animating placeholders if any blurred textures are still loading
        if self.right_blurred_textures.iter().any(|t| t.is_none()) {
            ctx.request_repaint_after(Duration::from_millis(16)); 
        }
        
        // Non-blocking: accept any blurred images that are ready and upload textures
        if let Some(rx) = &self.blurred_rx {
            let mut received_any = false;
            while let Ok((job_id, idx, rgba)) = rx.try_recv() {
                if job_id == self.blur_job_id {
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
                    let tex = ctx.load_texture(format!("right_first_blurred_{}", idx), color_image, TextureOptions::LINEAR);
                    if idx < self.right_blurred_textures.len() {
                        self.right_blurred_textures[idx] = Some(tex);
                        received_any = true;
                    }
                }
            }
            if received_any {
                ctx.request_repaint();
            }
        }
        
        // Debounced regeneration handler
        if let (Some(kind), Some(deadline)) = (self.pending_regen, self.regen_deadline) {
            if Instant::now() >= deadline {
                if self.profiling { println!("[profile] update: run scheduled {:?}", kind); }
                match kind {
                    RegenKind::Full => self.regenerate(ctx),
                    RegenKind::ImagesOnly => self.rebuild_textures_quick(ctx),
                }
                self.pending_regen = None;
                self.regen_deadline = None;
            } else {
                ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
        }
        
        // Top controls bar
        egui::TopBottomPanel::top("controls_top").show(ctx, |ui| {
            ui.heading("Poly Cue tag generator");
            // Row 1: Core controls
            ui.horizontal_wrapped(|ui| {
                ui.label("Count:");
                let mut count_i = self.count as i32;
                let max_count = self.max_possible_count as i32;
                if ui.add(egui::Slider::new(&mut count_i, SliderConfig::COUNT_MIN..=max_count)).changed() {
                    let new_count = count_i as usize;
                    if new_count != self.count {
                        self.count = new_count;
                        self.schedule_regen(RegenKind::Full, 200);
                    }
                }
                ui.label(format!("(max: {})", self.max_possible_count));
                ui.separator();
                ui.label("Sides:");
                let mut sides_i = self.sides as i32;
                if ui.add(egui::Slider::new(&mut sides_i, SliderConfig::SIDES_MIN..=SliderConfig::SIDES_MAX)).changed() {
                    let new_sides = sides_i as usize;
                    if new_sides != self.sides {
                        self.sides = new_sides;
                        self.update_max_possible_count();
                        // Clamp count to new maximum
                        self.count = self.count.min(self.max_possible_count);
                        self.schedule_regen(RegenKind::Full, 200);
                    }
                }
                ui.separator();
                ui.label(format!("ΔE threshold (auto): {:.1}", self.threshold));
                if ui.button("Regenerate").clicked() {
                    self.regenerate(ctx);
                }
                if ui.button("Save All Separate").clicked() {
                    self.save_current_tags();
                }
                if ui.button("Save All Together").clicked() {
                    self.save_current_tags_together();
                }
            });

            // Row 2: Visual controls
            ui.horizontal_wrapped(|ui| {
                // Center dot toggle + size
                let mut center_cb = self.center_dot;
                if ui.checkbox(&mut center_cb, "center dot").changed() {
                    self.center_dot = center_cb;
                    self.schedule_regen(RegenKind::ImagesOnly, 50);
                }
                ui.add_enabled_ui(self.center_dot, |ui| {
                    ui.label("Center dot size (%):");
                    let mut sz = self.center_dot_size_pct;
                    if ui.add(egui::Slider::new(&mut sz, SliderConfig::CENTER_DOT_MIN..=SliderConfig::CENTER_DOT_MAX).step_by(SliderConfig::CENTER_DOT_STEP)).changed() {
                        self.center_dot_size_pct = sz;
                        self.schedule_regen(RegenKind::ImagesOnly, 50);
                    }
                });

                ui.separator();

                // Gradient dot toggle + size (independent)
                let mut gd = self.gradient_dot;
                if ui.checkbox(&mut gd, "gradient dot").changed() {
                    self.gradient_dot = gd;
                    self.schedule_regen(RegenKind::ImagesOnly, 50);
                }
                ui.add_enabled_ui(self.gradient_dot, |ui| {
                    ui.label("Gradient dot size (%):");
                    let mut gsz = self.gradient_dot_size_pct;
                    if ui.add(egui::Slider::new(&mut gsz, SliderConfig::GRADIENT_DOT_MIN..=SliderConfig::GRADIENT_DOT_MAX).step_by(SliderConfig::GRADIENT_DOT_STEP)).changed() {
                        self.gradient_dot_size_pct = gsz;
                        self.schedule_regen(RegenKind::ImagesOnly, 50);
                    }
                });

                ui.separator();
                ui.label("Tag resolution:");
                let mut pw = self.preview_max_width as f32;
                if ui.add(egui::Slider::new(&mut pw, SliderConfig::RESOLUTION_MIN..=SliderConfig::RESOLUTION_MAX)).changed() {
                    self.preview_max_width = pw.round() as u32;
                    self.rebuild_textures_quick(ctx);
                }
            });

            // Row 3: Layout controls
            ui.horizontal_wrapped(|ui| {
                ui.label("Columns:");
                let mut cols_i = self.columns as i32;
                if ui.add(egui::Slider::new(&mut cols_i, SliderConfig::COLUMNS_MIN..=SliderConfig::COLUMNS_MAX)).changed() {
                    self.columns = cols_i as usize;
                }
                ui.separator();
                let mut prof = self.profiling;
                if ui.checkbox(&mut prof, "profiling logs").changed() {
                    self.profiling = prof;
                    if self.profiling { println!("[profile] enabled"); } else { println!("[profile] disabled"); }
                }
                ui.separator();
                let mut defer = self.defer_high_res;
                if ui.checkbox(&mut defer, "defer high-res").on_hover_text("Skip rendering high-res images during interactive changes; still renders on Save").changed() {
                    self.defer_high_res = defer;
                }
            });
        });

        // Left half: tags grid
        let panel_response = egui::SidePanel::left("tags_left").resizable(true).default_width(800.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let cols = self.columns.max(1);
                let avail = ui.available_width();
                let spacing = ui.spacing().item_spacing.x;
                let tile_w = ((avail - spacing * ((cols as f32) - 1.0)) / (cols as f32))
                    .floor()
                    .max(32.0);
                self.last_left_tile_w = tile_w;
                let mut i = 0;
                while i < self.textures.len() {
                    ui.horizontal(|ui| {
                        for _ in 0..cols {
                            if i >= self.textures.len() { break; }
                            let tex = &self.textures[i];
                            ui.add(egui::Image::new((tex.id(), egui::Vec2::new(tile_w, tile_w))));
                            i += 1;
                        }
                    });
                }
            });
        });
        
        // Check if panel width changed and trigger regeneration
        let current_width = panel_response.response.rect.width();
        if (current_width - self.last_panel_width).abs() > 1.0 {
            self.last_panel_width = current_width;
            self.schedule_regen(RegenKind::ImagesOnly, 100);
        }

        // Right half: placeholder for future graphics/content
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Section: All tags monochrome half-size
                ui.label("Monochrome (half-size)");
                let mono_w = (self.last_left_tile_w * 0.5).max(2.0);
                ui.horizontal_wrapped(|ui| {
                    for tex in &self.right_mono_textures {
                        ui.add(egui::Image::new((tex.id(), egui::Vec2::new(mono_w, mono_w))));
                    }
                });
                ui.separator();

                // Section: First tag scaled variants
                ui.label("First tag scaled");
                let scales: [f32; 18] = [
                    0.5, 0.4, 0.3, 0.2, 0.15, 0.14, 0.13, 0.12, 0.1,
                    0.09, 0.08, 0.07, 0.06, 0.05, 0.04, 0.03, 0.02, 0.01,
                ];
                ui.horizontal_wrapped(|ui| {
                    for (i, tex) in self.right_first_scaled_textures.iter().enumerate() {
                        let w = (self.last_left_tile_w * scales[i]).max(2.0);
                        ui.add(egui::Image::new((tex.id(), egui::Vec2::new(w, w))));
                    }
                });
                ui.separator();

                // Section: Heavily blurred first tag
                ui.label("First tag blurred (levels)");
                let w = self.last_left_tile_w.max(2.0);
                ui.horizontal_wrapped(|ui| {
                    let time = ctx.input(|i| i.time) as f32;
                    for (i, ot) in self.right_blurred_textures.iter().enumerate() {
                        if let Some(tex) = ot {
                            ui.add(egui::Image::new((tex.id(), egui::Vec2::new(w, w))));
                        } else {
                            // Animated ripple placeholder: fade up/down with a phase offset per index
                            let phase = time * 2.0 + (i as f32) * 0.6;
                            let alpha = 0.35 + 0.20 * phase.sin(); // 0.15..0.55
                            let (rect, _resp) = ui.allocate_exact_size(egui::Vec2::new(w, w), egui::Sense::hover());
                            let color = egui::Color32::from_rgba_unmultiplied(200, 200, 200, (alpha * 255.0) as u8);
                            ui.painter().rect(rect, 8.0, color, (1.0, egui::Color32::from_rgba_unmultiplied(160,160,160, (alpha*255.0) as u8)));
                        }
                    }
                });
            });
        });
    }
}
