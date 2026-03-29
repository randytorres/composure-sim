//! Composure Curve: degradation/recovery analysis with archetype classification.
//!
//! Ported from `sim/src/results/scoring/ProfileBuilder.ts`.
//! The Composure Curve measures how a system (person, organism, marketing campaign)
//! degrades under stress and recovers after stress removal.

use serde::{Deserialize, Serialize};

/// A point on the composure curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposurePoint {
    /// Time step.
    pub t: usize,
    /// Health/performance index at this time.
    pub value: f64,
    /// Event type (stress onset, error, recovery, etc.)
    pub event_type: EventType,
}

/// Event type at a composure curve point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Normal,
    Error,
    StressOnset,
    StressRemoval,
    Recovery,
    Custom(String),
}

/// Behavioral archetype derived from the composure curve shape.
///
/// Originally from SIM: steady, cliff_faller, phoenix, oscillator, plateau, surge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Archetype {
    /// Consistent performer. Low variance, stable trend.
    Steady,
    /// Strong start, sudden collapse under sustained stress.
    CliffFaller,
    /// Fast drop followed by strong recovery. Resilient.
    Phoenix,
    /// Alternating high/low performance. Inconsistent.
    Oscillator,
    /// Flat line. Neither improving nor degrading.
    Plateau,
    /// Upward trend. Improving under pressure.
    Surge,
}

impl Archetype {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Steady => "Steady",
            Self::CliffFaller => "Cliff Faller",
            Self::Phoenix => "Phoenix",
            Self::Oscillator => "Oscillator",
            Self::Plateau => "Plateau",
            Self::Surge => "Surge",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Steady => "Consistent performance under pressure. Reliable and predictable.",
            Self::CliffFaller => "Strong initial performance that collapses under sustained stress.",
            Self::Phoenix => "Performance drops under stress but recovers strongly. Resilient.",
            Self::Oscillator => "Alternating highs and lows. Inconsistent under pressure.",
            Self::Plateau => "Flat performance regardless of conditions. Neither improving nor degrading.",
            Self::Surge => "Performance improves under pressure. Thrives in challenging conditions.",
        }
    }
}

/// Metrics derived from the composure curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposureMetrics {
    /// Linear trend slope (positive = improving, negative = degrading).
    pub slope: f64,
    /// Variance of the health index over time.
    pub variance: f64,
    /// Peak health index value.
    pub peak: f64,
    /// Trough (minimum) health index value.
    pub trough: f64,
    /// Recovery half-life: time steps to recover 50% after a drop. `None` if no recovery observed.
    pub recovery_half_life: Option<usize>,
    /// Residual damage: how much performance is permanently lost after stress.
    pub residual_damage: f64,
    /// Break point: time step where performance first crosses below a failure threshold. `None` if never.
    pub break_point: Option<usize>,
}

/// Full composure curve analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposureCurve {
    /// The timeline of composure points.
    pub timeline: Vec<ComposurePoint>,
    /// Classified archetype.
    pub archetype: Archetype,
    /// Derived metrics.
    pub metrics: ComposureMetrics,
}

/// Analyze a trajectory (sequence of health index values) into a composure curve.
///
/// This is the main entry point. Takes a sequence of scalar health index values
/// (from Monte Carlo mean trajectory or a single path) and returns full analysis.
pub fn analyze_composure(values: &[f64], failure_threshold: f64) -> ComposureCurve {
    let timeline: Vec<ComposurePoint> = values
        .iter()
        .enumerate()
        .map(|(t, &v)| ComposurePoint {
            t,
            value: v,
            event_type: EventType::Normal,
        })
        .collect();

    let slope = linear_slope(values);
    let variance = compute_variance(values);
    let peak = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let trough = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let break_point = values.iter().position(|&v| v < failure_threshold);
    let recovery_half_life = compute_recovery_half_life(values);
    let residual_damage = compute_residual_damage(values);

    let archetype = classify_archetype_from_stats(slope, variance, values);

    let metrics = ComposureMetrics {
        slope,
        variance,
        peak,
        trough,
        recovery_half_life,
        residual_damage,
        break_point,
    };

    ComposureCurve {
        timeline,
        archetype,
        metrics,
    }
}

/// Classify archetype from a raw trajectory.
///
/// Public API for consumers who want just the archetype without full analysis.
pub fn classify_archetype(values: &[f64]) -> Archetype {
    if values.len() < 3 {
        return Archetype::Steady;
    }
    let slope = linear_slope(values);
    let variance = compute_variance(values);
    classify_archetype_from_stats(slope, variance, values)
}

/// Archetype classification logic.
/// Adapted from `sim/src/results/scoring/ProfileBuilder.ts` `classifyComposure()`.
/// Thresholds calibrated for [0,1] normalized state dimensions.
fn classify_archetype_from_stats(slope: f64, variance: f64, values: &[f64]) -> Archetype {
    if values.len() < 3 {
        return Archetype::Steady;
    }

    // Check for Phoenix: drop followed by recovery (check first — distinctive shape)
    if has_recovery_surge(values) {
        return Archetype::Phoenix;
    }

    // Check for Surge: meaningful upward trend
    // For [0,1] data over ~50-200 points, slope > 0.005 is a clear upward trend
    if slope > 0.005 {
        return Archetype::Surge;
    }

    // High variance paths (alternating or collapsing)
    // For [0,1] data, variance > 0.01 indicates significant instability
    if variance > 0.01 {
        let mid = values.len() / 2;
        let first_half = &values[..mid];
        let second_half = &values[mid..];
        let first_var = compute_variance(first_half);
        let first_avg = average(first_half);
        let second_avg = average(second_half);

        // Cliff Faller: stable first half, collapse in second half
        if first_var < 0.005 && second_avg < first_avg * 0.7 {
            return Archetype::CliffFaller;
        }

        return Archetype::Oscillator;
    }

    // Low variance + flat slope = Plateau
    if slope.abs() < 0.001 && variance < 0.005 {
        return Archetype::Plateau;
    }

    Archetype::Steady
}

/// Detect a recovery surge pattern (Phoenix archetype).
/// Requires: a SINGLE significant drop (>20% from peak) in the first half,
/// followed by sustained recovery (>60% of drop recovered) in the second half.
/// Must NOT be oscillating (high variance disqualifies — check variance first).
fn has_recovery_surge(values: &[f64]) -> bool {
    if values.len() < 8 {
        return false;
    }

    // Oscillating patterns should NOT be Phoenix.
    // If variance is high, let the oscillator/cliff-faller logic handle it.
    let variance = compute_variance(values);
    if variance > 0.04 {
        // Could still be Phoenix if there's a clear single-trough shape.
        // Check: does the trough occur in a concentrated region (not spread out)?
        // Count how many points are below the mean — if >40%, it's oscillating, not Phoenix.
        let mean = average(values);
        let below_mean_ratio = values.iter().filter(|&&v| v < mean).count() as f64 / values.len() as f64;
        if below_mean_ratio > 0.45 {
            return false;
        }
    }

    // Find peak in first 60% of the series
    let search_end = (values.len() as f64 * 0.6) as usize;
    let (peak_idx, peak_val) = values[..search_end]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();

    // Find trough after peak
    let (trough_offset, trough_val) = values[peak_idx..]
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();
    let trough_idx = peak_idx + trough_offset;

    let drop = peak_val - trough_val;
    if drop < 0.15 {
        return false; // Not a meaningful drop
    }

    // Trough must be before the last 20% of the series (need room to recover)
    if trough_idx >= (values.len() as f64 * 0.8) as usize {
        return false;
    }

    // Check recovery: post-trough values must recover significantly
    let post_trough = &values[trough_idx..];
    let recovery_peak = post_trough.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let recovery = recovery_peak - trough_val;

    recovery > drop * 0.6
}

/// Least-squares linear regression slope.
/// Ported from `ProfileBuilder.ts` `linearSlope()`.
pub fn linear_slope(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    if n < 2.0 {
        return 0.0;
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;

    for (i, &y) in values.iter().enumerate() {
        let x = i as f64;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
    }

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-12 {
        return 0.0;
    }

    (n * sum_xy - sum_x * sum_y) / denom
}

/// Population variance.
/// Ported from `ProfileBuilder.ts` `computeVariance()`.
pub fn compute_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = average(values);
    values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
}

/// Arithmetic mean.
pub fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Compute recovery half-life: time steps from trough to recovering 50% of the drop.
fn compute_recovery_half_life(values: &[f64]) -> Option<usize> {
    if values.len() < 4 {
        return None;
    }

    // Find global trough
    let (trough_idx, &trough_val) = values
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?;

    // Find pre-trough peak
    let pre_peak = values[..=trough_idx]
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let drop = pre_peak - trough_val;
    if drop < 0.05 {
        return None; // No meaningful drop
    }

    let half_recovery_target = trough_val + drop * 0.5;

    // Find first time after trough that crosses half-recovery
    for (i, &v) in values[trough_idx..].iter().enumerate() {
        if v >= half_recovery_target {
            return Some(i);
        }
    }

    None
}

/// Compute residual damage: difference between pre-stress peak and post-stress stable level.
fn compute_residual_damage(values: &[f64]) -> f64 {
    if values.len() < 4 {
        return 0.0;
    }

    let first_quarter = &values[..values.len() / 4];
    let last_quarter = &values[values.len() * 3 / 4..];

    let initial = average(first_quarter);
    let final_level = average(last_quarter);

    (initial - final_level).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steady_classification() {
        // Truly flat line — zero slope, zero variance
        let values: Vec<f64> = (0..50).map(|_| 0.5).collect();
        let arch = classify_archetype(&values);
        assert!(arch == Archetype::Steady || arch == Archetype::Plateau,
            "Expected Steady or Plateau for flat line, got {:?}", arch);
    }

    #[test]
    fn test_surge_classification() {
        // Very strong upward trend (slope >> 0.02)
        let values: Vec<f64> = (0..50).map(|i| 0.1 + i as f64 * 0.015).collect();
        assert_eq!(classify_archetype(&values), Archetype::Surge);
    }

    #[test]
    fn test_phoenix_classification() {
        // Start high, sharp drop, strong recovery
        let mut values = Vec::new();
        for _ in 0..10 { values.push(0.9); }             // Stable at 0.9
        for i in 0..10 { values.push(0.9 - i as f64 * 0.05); } // Drop to 0.4
        for i in 0..30 { values.push(0.4 + i as f64 * 0.018); } // Recover to ~0.94
        assert_eq!(classify_archetype(&values), Archetype::Phoenix);
    }

    #[test]
    fn test_cliff_faller_classification() {
        // Very stable first half, dramatic collapse second half
        let mut values = Vec::new();
        for _ in 0..25 { values.push(0.9); }              // Rock stable at 0.9
        for i in 0..25 { values.push(0.9 - i as f64 * 0.03); } // Collapse to 0.15
        assert_eq!(classify_archetype(&values), Archetype::CliffFaller);
    }

    #[test]
    fn test_linear_slope() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((linear_slope(&values) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_variance() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((compute_variance(&values) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_composure_analysis() {
        // Strong upward trend for Surge
        let values: Vec<f64> = (0..100).map(|i| 0.1 + (i as f64 * 0.008)).collect();
        let result = analyze_composure(&values, 0.3);
        assert_eq!(result.archetype, Archetype::Surge);
        assert!(result.metrics.slope > 0.0);
        assert_eq!(result.timeline.len(), 100);
    }

    #[test]
    fn test_short_series() {
        assert_eq!(classify_archetype(&[0.5]), Archetype::Steady);
        assert_eq!(classify_archetype(&[0.5, 0.6]), Archetype::Steady);
    }

    #[test]
    fn test_oscillator_classification() {
        // High variance with no clear drop-then-recover shape
        // Use a sine-like pattern that oscillates without a single trough
        let values: Vec<f64> = (0..50).map(|i| {
            0.5 + 0.3 * (i as f64 * 0.5).sin()
        }).collect();
        assert_eq!(classify_archetype(&values), Archetype::Oscillator);
    }
}
