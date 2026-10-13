/// Stroke smoothing algorithms for freehand drawing (Chaikin subdivision and distance filtering)

pub fn distance_filter(points: &[[f32; 2]], min_dist: f32) -> Vec<[f32; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut filtered = Vec::with_capacity(points.len());
    filtered.push(points[0]);
    let min_dist_sq = min_dist * min_dist;

    for &p in &points[1..points.len() - 1] {
        let last = filtered.last().unwrap();
        let dx = p[0] - last[0];
        let dy = p[1] - last[1];
        if dx * dx + dy * dy >= min_dist_sq {
            filtered.push(p);
        }
    }

    if let Some(&last_original) = points.last() {
        filtered.push(last_original);
    }

    filtered
}

pub fn apply_chaikin(points: &[[f32; 2]], iterations: u32) -> Vec<[f32; 2]> {
    if points.len() <= 2 || iterations == 0 {
        return points.to_vec();
    }

    let mut current = points.to_vec();

    for _ in 0..iterations {
        if current.len() <= 2 {
            break;
        }

        let mut next = Vec::with_capacity(current.len() * 2);
        next.push(current[0]);

        for i in 0..current.len() - 1 {
            let p0 = current[i];
            let p1 = current[i + 1];

            let q = [0.75 * p0[0] + 0.25 * p1[0], 0.75 * p0[1] + 0.25 * p1[1]];
            let r = [0.25 * p0[0] + 0.75 * p1[0], 0.25 * p0[1] + 0.75 * p1[1]];

            next.push(q);
            next.push(r);
        }

        next.push(*current.last().unwrap());
        current = next;
    }

    current
}

pub fn smooth_stroke(points: &[[f32; 2]], intensity: u32) -> Vec<[f32; 2]> {
    if points.len() <= 2 || intensity == 0 {
        return points.to_vec();
    }

    let dist_threshold = 2.0 + (intensity as f32) * 1.5;
    let filtered = distance_filter(points, dist_threshold);
    let chaikin_iters = intensity.clamp(1, 4);
    apply_chaikin(&filtered, chaikin_iters)
}
