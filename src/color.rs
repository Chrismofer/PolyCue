use palette::{rgb::Srgb, FromColor, Lab};
use image::Rgb;
use rand::{seq::SliceRandom, thread_rng, Rng};

/// CIE76 distance calculation for perceptually uniform color differences
pub fn delta_e(a: Lab, b: Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

/// Convert sRGB u8 values to CIE Lab color space
pub fn srgb_u8_to_lab(rgb: Rgb<u8>) -> Lab {
    let srgb_f = Srgb::new(
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    );
    Lab::from_color(srgb_f.into_linear())
}

/// Generate a coarse grid of sRGB colors (6 levels per channel = 216 candidates)
pub fn candidate_srgb_grid() -> Vec<Rgb<u8>> {
    let levels: [u8; 6] = [16, 64, 112, 160, 208, 255];
    let mut v = Vec::with_capacity(216);
    for &r in &levels {
        for &g in &levels {
            for &b in &levels {
                v.push(Rgb([r, g, b]));
            }
        }
    }
    v
}

/// Pick distinct colors based on strict threshold requirements
pub fn pick_distinct_strict(
    labs: &[Lab],
    order: &[usize],
    threshold: f32,
    limit: usize,
) -> Vec<usize> {
    let mut picked_idx: Vec<usize> = Vec::with_capacity(limit);
    let mut picked_labs: Vec<Lab> = Vec::with_capacity(limit);
    for &i in order {
        let ok = picked_labs.iter().all(|&pl| delta_e(pl, labs[i]) >= threshold);
        if ok {
            picked_idx.push(i);
            picked_labs.push(labs[i]);
            if picked_idx.len() >= limit { break; }
        }
    }
    picked_idx
}

/// Compute the maximum feasible color separation threshold for a given set
pub fn compute_max_threshold_and_colors_from_pool(
    filtered: &[Rgb<u8>],
    labs: &[Lab],
    total: usize,
) -> (f32, Vec<Rgb<u8>>) {
    let mut rng = thread_rng();
    
    // Determine upper bound by sampling for max pairwise ΔE
    let mut max_d = 0.0f32;
    for _ in 0..512 {
        let i = rng.gen_range(0..labs.len());
        let j = rng.gen_range(0..labs.len());
        if i == j { continue; }
        let d = delta_e(labs[i], labs[j]);
        if d > max_d { max_d = d; }
    }
    
    let mut lo = 0.0f32;
    let mut hi = max_d;
    let mut best_thr = 0.0f32;
    let mut best_idxs: Vec<usize> = Vec::new();

    // Binary search for highest feasible threshold
    for _ in 0..14 {
        let mid = (lo + hi) * 0.5;
        let mut feasible = false;
        let mut attempt_best: Vec<usize> = Vec::new();
        
        // Try a few shuffled orders per threshold
        for _ in 0..4 {
            let mut order: Vec<usize> = (0..filtered.len()).collect();
            order.shuffle(&mut rng);
            let picked = pick_distinct_strict(labs, &order, mid, total);
            if picked.len() >= total {
                feasible = true;
                attempt_best = picked;
                break;
            }
        }
        
        if feasible {
            best_thr = mid;
            best_idxs = attempt_best;
            lo = mid;
        } else {
            hi = mid;
        }
    }

    // Build color list from best indices
    if best_idxs.len() < total {
        let mut order: Vec<usize> = (0..filtered.len()).collect();
        order.shuffle(&mut rng);
        best_idxs = pick_distinct_strict(labs, &order, best_thr, total);
    }
    
    let mut colors: Vec<Rgb<u8>> = best_idxs.into_iter().map(|i| filtered[i]).collect();
    colors.truncate(total);
    (best_thr, colors)
}

/// Compute pairwise distance matrix for Lab colors
pub fn pairwise_delta_matrix(labs: &[Lab]) -> Vec<f32> {
    let n = labs.len();
    let mut dm = vec![0.0f32; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = delta_e(labs[i], labs[j]);
            dm[i * n + j] = d;
            dm[j * n + i] = d;
        }
    }
    dm
}

/// Find minimum distance within a group using the distance matrix
pub fn group_min(dm: &[f32], n: usize, group: &[usize]) -> f32 {
    let mut min_d = f32::INFINITY;
    for i in 0..group.len() {
        for j in (i + 1)..group.len() {
            let d = dm[group[i] * n + group[j]];
            if d < min_d {
                min_d = d;
            }
        }
    }
    min_d
}

/// Reorder colors to alternate bright and dark for maximum adjacent contrast
pub fn reorder_bright_dark_alternating(colors: &mut Vec<Rgb<u8>>) {
    let n = colors.len();
    if n < 2 || n % 2 != 0 {
        return;
    }
    
    let mut with_l: Vec<(Rgb<u8>, f32)> = colors
        .iter()
        .copied()
        .map(|c| (c, srgb_u8_to_lab(c).l))
        .collect();
    
    // Sort by brightness (Lab L) descending: brightest first
    with_l.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    let half = n / 2;
    let brights: Vec<Rgb<u8>> = with_l[..half].iter().map(|(c, _)| *c).collect();
    let darks: Vec<Rgb<u8>> = with_l[half..].iter().map(|(c, _)| *c).collect();
    
    let mut reordered: Vec<Rgb<u8>> = Vec::with_capacity(n);
    for i in 0..half {
        reordered.push(brights[i]);
        reordered.push(darks[i]);
    }
    *colors = reordered;
}
