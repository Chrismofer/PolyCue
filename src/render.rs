use image::{ImageBuffer, Rgb};
use crate::color::{pairwise_delta_matrix, group_min};
use palette::Lab;
use rand::{thread_rng, Rng};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Group colors into optimal arrangements using Monte Carlo optimization
pub fn group_colors_into_groups_monte_carlo(
    colors: Vec<Rgb<u8>>,
    labs: Vec<Lab>,
    tag_count: usize,
    group_size: usize,
    iters: usize,
) -> Vec<Vec<Rgb<u8>>> {
    let n = colors.len();
    assert_eq!(n, tag_count * group_size);
    let dm = pairwise_delta_matrix(&labs);

    // Greedy initialization: for each group, pick the farthest pair, then add items maximizing min distance to group
    let mut remaining: Vec<usize> = (0..n).collect();
    let mut groups: Vec<Vec<usize>> = Vec::with_capacity(tag_count);

    while !remaining.is_empty() {
        // Seed with farthest pair
        let mut best_pair = (remaining[0], remaining[1], -1.0f32);
        for i in 0..remaining.len() {
            for j in (i + 1)..remaining.len() {
                let a = remaining[i];
                let b = remaining[j];
                let d = dm[a * n + b];
                if d > best_pair.2 {
                    best_pair = (a, b, d);
                }
            }
        }
        
        let (a, b, _d) = best_pair;
        let mut group = vec![a, b];
        remaining.retain(|&x| x != a && x != b);
        
        // Fill the rest of the group
        while group.len() < group_size {
            // choose c maximizing min distance to current group
            let mut best_c = remaining[0];
            let mut best_score = -1.0f32;
            for &c in &remaining {
                // compute min distance from c to group
                let mut m = f32::INFINITY;
                for &g in &group {
                    let d = dm[g * n + c];
                    if d < m { m = d; }
                }
                if m > best_score {
                    best_score = m;
                    best_c = c;
                }
            }
            group.push(best_c);
            remaining.retain(|&x| x != best_c);
        }
        groups.push(group);
    }

    // Monte Carlo refinement: swap one color between two groups if it improves total score
    let mut rng = thread_rng();
    let score_group = |g: &Vec<usize>| -> f32 { group_min(&dm, n, g) };

    for _ in 0..iters {
        if tag_count < 2 { break; }
        let i = rng.gen_range(0..tag_count);
        let mut j = rng.gen_range(0..tag_count);
        if i == j { j = (j + 1) % tag_count; }
        let ia = rng.gen_range(0..group_size);
        let jb = rng.gen_range(0..group_size);

        let old_i = groups[i].clone();
        let old_j = groups[j].clone();
        let old_score = score_group(&old_i) + score_group(&old_j);

        // try swap
        groups[i][ia] = old_j[jb];
        groups[j][jb] = old_i[ia];
        let new_score = score_group(&groups[i]) + score_group(&groups[j]);

        if new_score + f32::EPSILON >= old_score {
            // accept if not worse
        } else {
            // revert
            groups[i] = old_i;
            groups[j] = old_j;
        }
    }

    // Map back to RGB triplets
    groups
        .into_iter()
        .map(|g| g.into_iter().map(|idx| colors[idx]).collect::<Vec<_>>())
        .collect()
}

/// Draw a filled triangle using scanline rasterization
pub fn draw_filled_triangle(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, a: Point, b: Point, c: Point, color: Rgb<u8>) {
    let width = img.width();
    let height = img.height();
    
    // Sort vertices by y coordinate
    let mut pts = [a, b, c];
    pts.sort_by_key(|p| p.y);
    let (p0, p1, p2) = (pts[0], pts[1], pts[2]);

    let interp = |p0: Point, p1: Point, y: i32| -> f32 {
        if p1.y == p0.y {
            p0.x as f32
        } else {
            p0.x as f32 + (p1.x - p0.x) as f32 * ((y - p0.y) as f32 / (p1.y - p0.y) as f32)
        }
    };

    let mut draw_span = |y: i32, x0: i32, x1: i32| {
        if y < 0 || y >= height as i32 {
            return;
        }
        let (mut xa, mut xb) = (x0.min(x1), x0.max(x1));
        xa = xa.max(0);
        xb = xb.min(width as i32 - 1);
        for x in xa..=xb {
            img.put_pixel(x as u32, y as u32, color);
        }
    };

    // Upper part p0->p1 and p0->p2
    for y in p0.y..=p1.y {
        if y < 0 || y >= height as i32 { continue; }
        let xa = interp(p0, p2, y).round() as i32;
        let xb = interp(p0, p1, y).round() as i32;
        draw_span(y, xa, xb);
    }
    
    // Lower part p1->p2 and p0->p2
    for y in (p1.y + 1)..=p2.y {
        if y < 0 || y >= height as i32 { continue; }
        let xa = interp(p0, p2, y).round() as i32;
        let xb = interp(p1, p2, y).round() as i32;
        draw_span(y, xa, xb);
    }
}

/// Draw a polygonal marker with optional center and gradient dots
#[allow(clippy::too_many_arguments)]
pub fn draw_marker_polygon(
    width: u32, 
    height: u32, 
    sides: usize, 
    colors: &[Rgb<u8>], 
    center_dot: bool, 
    center_dot_size_pct: f32, 
    gradient_dot: bool, 
    gradient_dot_size_pct: f32
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(width, height, Rgb([255, 255, 255]));

    let w = width as f32;
    let h_img = height as f32;
    
    // Draw centered on the full canvas with even padding
    let margin = 0.08f32 * w.min(h_img);
    let radius = ((w - 2.0 * margin) * 0.5)
        .min((h_img - 2.0 * margin) * 0.5)
        .max(1.0);
    let cx = w * 0.5;
    let cy = h_img * 0.5;
    let angle_step = std::f32::consts::TAU / (sides as f32);
    let start_angle = -std::f32::consts::FRAC_PI_2; // point up

    let mut verts: Vec<Point> = Vec::with_capacity(sides);
    for i in 0..sides {
        let a = start_angle + angle_step * (i as f32);
        let x = cx + radius * a.cos();
        let y = cy + radius * a.sin();
        verts.push(Point { x: x.round() as i32, y: y.round() as i32 });
    }
    let centroid = Point { x: cx.round() as i32, y: cy.round() as i32 };

    // Draw colored triangular segments
    for i in 0..sides {
        let v0 = verts[i];
        let v1 = verts[(i + 1) % sides];
        let color = colors[i % colors.len()];
        draw_filled_triangle(&mut img, centroid, v0, v1, color);
    }

    // Optional center dot (solid black circle)
    if center_dot {
        let pct = (center_dot_size_pct / 100.0).clamp(0.01, 0.5);
        let r = ((w.min(h_img)) * pct * 0.5).max(1.0);
        let r2 = r * r;
        let x0 = ((cx - r).floor() as i32).max(0);
        let y0 = ((cy - r).floor() as i32).max(0);
        let x1 = ((cx + r).ceil() as i32).min((width as i32) - 1);
        let y1 = ((cy + r).ceil() as i32).min((height as i32) - 1);
        
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32) - cx;
                let dy = (y as f32) - cy;
                if dx * dx + dy * dy <= r2 {
                    img.put_pixel(x as u32, y as u32, Rgb([0, 0, 0]));
                }
            }
        }
    }
    
    // Optional gradient dot (Gaussian fade to white)
    if gradient_dot {
        let pct_g = (gradient_dot_size_pct / 100.0).clamp(0.01, 0.5);
        let rg = ((w.min(h_img)) * pct_g * 0.5).max(1.0);
        let rg2 = rg * rg;
        let x0 = ((cx - rg).floor() as i32).max(0);
        let y0 = ((cy - rg).floor() as i32).max(0);
        let x1 = ((cx + rg).ceil() as i32).min((width as i32) - 1);
        let y1 = ((cy + rg).ceil() as i32).min((height as i32) - 1);
        let sigma = (rg * 0.7).max(0.5);
        let two_sigma2 = 2.0 * sigma * sigma;
        
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32) - cx;
                let dy = (y as f32) - cy;
                let dist2 = dx * dx + dy * dy;
                if dist2 <= rg2 {
                    let alpha = (-dist2 / two_sigma2).exp();
                    if alpha > 0.001 {
                        let p = img.get_pixel_mut(x as u32, y as u32);
                        let (r0, g0, b0) = (p[0] as f32, p[1] as f32, p[2] as f32);
                        let inv = 1.0 - alpha;
                        let r1 = (255.0 * alpha + r0 * inv).round().clamp(0.0, 255.0) as u8;
                        let g1 = (255.0 * alpha + g0 * inv).round().clamp(0.0, 255.0) as u8;
                        let b1 = (255.0 * alpha + b0 * inv).round().clamp(0.0, 255.0) as u8;
                        *p = Rgb([r1, g1, b1]);
                    }
                }
            }
        }
    }

    img
}
