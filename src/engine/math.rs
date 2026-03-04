pub const CLAMP_MIN: f32 = 0.0;
pub const CLAMP_MAX: f32 = 100.0;

/// Clamp to [0,100], also converts -0.0 to 0.0.
pub fn clamp01(x: f32) -> f32 {
    let y = x.clamp(CLAMP_MIN, CLAMP_MAX);
    if y == 0.0 {
        0.0
    } else {
        y
    }
}

/// Returns true if x is finite and within clamp bounds (inclusive).
pub fn is_valid_state_value(x: f32) -> bool {
    x.is_finite() && (CLAMP_MIN..=CLAMP_MAX).contains(&x)
}

/// Compute influence sum over incoming edges using normalized parent values.
/// influence = Σ(w * (parent/100.0))
pub fn compute_influence(incoming: &[(usize, f32)], values: &[f32]) -> f32 {
    let mut influence = 0.0_f32;
    for (parent_idx, weight) in incoming {
        if let Some(parent_value) = values.get(*parent_idx) {
            influence += weight * (*parent_value / 100.0);
        }
    }
    influence
}

/// Apply one Euler-style update step:
/// next = clamp(current + bias + influence*influence_scale + noise)
pub fn apply_update(
    current: f32,
    bias: f32,
    influence: f32,
    influence_scale: f32,
    noise: f32,
) -> f32 {
    clamp01(current + bias + influence * influence_scale + noise)
}
