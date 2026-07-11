//! Anomaly detection for corridor reliability metrics (#1783).
//!
//! This is a minimal, real implementation of the "rolling baseline /
//! statistical threshold" approach described in the issue: it does not yet
//! persist alerts or expose them over the API — that's follow-up work — but
//! the detection algorithm itself is genuine and tested.

/// A single detected anomaly in a corridor reliability time series.
#[derive(Debug, Clone, PartialEq)]
pub struct CorridorAnomaly {
    /// Index into the input slice where the anomaly was observed.
    pub index: usize,
    /// The observed value at `index` (e.g. success rate as a percentage).
    pub value: f64,
    /// Mean of the rolling baseline window immediately preceding `index`.
    pub baseline_mean: f64,
    /// Population standard deviation of that same baseline window.
    pub baseline_stddev: f64,
    /// Number of standard deviations `value` is from `baseline_mean`.
    pub deviation: f64,
}

/// Flags points in `values` that deviate from their preceding rolling
/// baseline by more than `threshold_stddev` standard deviations.
///
/// # Algorithm
/// For each point at index `i >= window`, compute the mean and population
/// standard deviation of the preceding `window` points. If the current
/// point's absolute deviation from that mean exceeds
/// `threshold_stddev * stddev`, it's flagged as an anomaly.
///
/// Points before the first full window are never flagged — there isn't
/// enough history yet to establish a baseline.
///
/// # Deduplication
/// A sustained deviation (e.g. a corridor that drops and stays down) is
/// only flagged once, at the point the deviation *begins*, rather than
/// once per affected data point. This is the "basic deduplication" called
/// for in the issue's acceptance criteria — full alert-lifecycle
/// deduplication (e.g. across alert restarts) is out of scope here.
#[must_use]
pub fn detect_reliability_anomalies(
    values: &[f64],
    window: usize,
    threshold_stddev: f64,
) -> Vec<CorridorAnomaly> {
    if window == 0 || values.len() <= window {
        return Vec::new();
    }

    let mut anomalies = Vec::new();
    let mut previously_anomalous_direction: Option<bool> = None; // Some(true) = above baseline

    for i in window..values.len() {
        let baseline = &values[i - window..i];
        let mean = baseline.iter().sum::<f64>() / window as f64;
        let variance =
            baseline.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window as f64;
        let stddev = variance.sqrt();

        let value = values[i];
        let diff = value - mean;
        let deviation = if stddev > 0.0 {
            diff.abs() / stddev
        } else if diff != 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let is_anomalous = deviation >= threshold_stddev;
        let direction = diff >= 0.0;

        if is_anomalous && previously_anomalous_direction != Some(direction) {
            anomalies.push(CorridorAnomaly {
                index: i,
                value,
                baseline_mean: mean,
                baseline_stddev: stddev,
                deviation,
            });
        }

        previously_anomalous_direction = if is_anomalous { Some(direction) } else { None };
    }

    anomalies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_single_injected_drop() {
        // Stable ~99% success rate, then a sharp one-off drop to 60%.
        let mut values = vec![99.0; 10];
        values.push(60.0);
        values.extend(vec![99.0; 3]);

        let anomalies = detect_reliability_anomalies(&values, 5, 3.0);

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].index, 10);
        assert_eq!(anomalies[0].value, 60.0);
    }

    #[test]
    fn does_not_flag_normal_fluctuation_within_threshold() {
        let values = vec![98.0, 99.0, 97.5, 98.5, 99.2, 98.1, 98.8, 99.0, 97.9, 98.3];

        let anomalies = detect_reliability_anomalies(&values, 5, 3.0);

        assert!(anomalies.is_empty());
    }

    #[test]
    fn returns_empty_when_there_is_not_enough_history_for_a_baseline() {
        let values = vec![99.0, 60.0, 99.0];

        assert!(detect_reliability_anomalies(&values, 5, 3.0).is_empty());
    }

    #[test]
    fn deduplicates_a_sustained_drop_into_a_single_alert() {
        // A wide window (10) means the rolling baseline absorbs the drop
        // slowly, so the second point in the drop (index 16) is *also*
        // anomalous on its own — this is what actually exercises dedup,
        // rather than relying on the baseline catching up immediately.
        let mut values = vec![99.0; 15];
        values.extend(vec![50.0; 5]);

        let anomalies = detect_reliability_anomalies(&values, 10, 3.0);

        assert_eq!(
            anomalies.len(),
            1,
            "a sustained drop should produce one alert, not one per point"
        );
        assert_eq!(anomalies[0].index, 15);
    }

    #[test]
    fn flags_a_spike_as_well_as_a_drop() {
        let mut values = vec![50.0; 10];
        values.push(95.0);
        values.extend(vec![50.0; 3]);

        let anomalies = detect_reliability_anomalies(&values, 5, 3.0);

        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].index, 10);
        assert_eq!(anomalies[0].value, 95.0);
    }
}
