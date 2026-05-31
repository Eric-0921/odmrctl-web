//! B-channel statistics computation from full 50-sample RALL? measurement vectors.
//!
//! Uses `RallFrame.measurements.lockin_B_X_mv` / `lockin_B_Y_mv` directly
//! rather than only the latest sample.

pub struct BChannelVectorStats {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

/// Compute mean, std, min, and max for a slice of f64 values.
/// Returns `None` if the input is empty or contains NaN/infinite values.
pub fn compute_vector_stats(values: &[f64]) -> Option<BChannelVectorStats> {
    if values.is_empty() {
        return None;
    }
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let n = values.len() as f64;
    let sum: f64 = values.iter().sum();
    let mean = sum / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some(BChannelVectorStats {
        mean,
        std,
        min,
        max,
    })
}

/// Aggregate B-channel statistics from a collection of frames.
/// Each frame contributes its 50-sample vector for X and Y channels.
/// Returns `None` if no frames contain valid data.
pub fn aggregate_b_channel_stats(
    b_x_vectors: &[Vec<f64>],
    b_y_vectors: &[Vec<f64>],
) -> Option<(BChannelVectorStats, BChannelVectorStats)> {
    let all_x: Vec<f64> = b_x_vectors
        .iter()
        .flat_map(|v| v.iter().copied())
        .collect();
    let all_y: Vec<f64> = b_y_vectors
        .iter()
        .flat_map(|v| v.iter().copied())
        .collect();

    let x_stats = compute_vector_stats(&all_x)?;
    let y_stats = compute_vector_stats(&all_y)?;
    Some((x_stats, y_stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_single_element() {
        let s = compute_vector_stats(&[5.0]).unwrap();
        assert!((s.mean - 5.0).abs() < 1e-9);
        assert!((s.std - 0.0).abs() < 1e-9);
        assert!((s.min - 5.0).abs() < 1e-9);
        assert!((s.max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn stats_uniform() {
        let v = vec![2.0; 50];
        let s = compute_vector_stats(&v).unwrap();
        assert!((s.mean - 2.0).abs() < 1e-9);
        assert!((s.std - 0.0).abs() < 1e-9);
    }

    #[test]
    fn stats_range_known() {
        let v: Vec<f64> = (1..=3).map(|x| x as f64).collect(); // [1,2,3]
        let s = compute_vector_stats(&v).unwrap();
        assert!((s.mean - 2.0).abs() < 1e-9);
        assert!((s.min - 1.0).abs() < 1e-9);
        assert!((s.max - 3.0).abs() < 1e-9);
        let expected_std = (2.0_f64 / 3.0_f64).sqrt();
        assert!((s.std - expected_std).abs() < 1e-9);
    }

    #[test]
    fn stats_empty_returns_none() {
        assert!(compute_vector_stats(&[]).is_none());
    }

    #[test]
    fn stats_nan_returns_none() {
        assert!(compute_vector_stats(&[1.0, f64::NAN, 3.0]).is_none());
    }

    #[test]
    fn stats_infinite_returns_none() {
        assert!(compute_vector_stats(&[1.0, f64::INFINITY, 3.0]).is_none());
    }

    #[test]
    fn aggregate_with_valid_vectors() {
        let bx = vec![vec![1.0, 2.0, 3.0]];
        let by = vec![vec![4.0, 5.0, 6.0]];
        let (xs, ys) = aggregate_b_channel_stats(&bx, &by).unwrap();
        assert!((xs.mean - 2.0).abs() < 1e-9);
        assert!((ys.mean - 5.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_empty_returns_none() {
        assert!(aggregate_b_channel_stats(&[], &[]).is_none());
    }
}
