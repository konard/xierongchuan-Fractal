//! Collection of methods for audio waveforms.
//!
//! This does not depend on the audio decoding backend, so it is available on
//! every platform.

use crate::utils::resample_slice;

/// Normalize the given waveform to have between 30 and 120 samples with a value
/// between 0 and 1.
///
/// All the samples in the waveform must be positive or negative. If they are
/// mixed, this will change the waveform because it uses the absolute value of
/// the sample.
///
/// If the waveform was empty, returns an empty vec.
///
/// Note that the number of required samples comes from MSC3246.
pub(crate) fn normalize_waveform(waveform: Vec<f64>) -> Vec<f32> {
    if waveform.is_empty() {
        return vec![];
    }

    let max = waveform
        .iter()
        .copied()
        .map(f64::abs)
        .reduce(f64::max)
        .expect("iterator should contain at least one value");

    // Normalize between 0 and 1, with the highest value as 1.
    let mut normalized = waveform
        .into_iter()
        .map(f64::abs)
        .map(|value| if max == 0.0 { value } else { value / max } as f32)
        .collect::<Vec<_>>();

    match normalized.len() {
        0..30 => normalized = resample_slice(&normalized, 30).into_owned(),
        30..120 => {}
        _ => normalized = resample_slice(&normalized, 120).into_owned(),
    }

    normalized
}
