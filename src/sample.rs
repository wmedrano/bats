//! Utilities for a single audio samples.
use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Result};

/// Contains an audio sample. This includes both left and right channels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    /// The audio data. The first half of the buffer contains the left
    /// channel and the second contains the right channel.
    buffer: Arc<Vec<f32>>,
}

impl Sample {
    /// Create a new sample with the given data for both the left and right channels.
    pub fn with_mono_data(data: &[f32]) -> Sample {
        match Sample::with_stereo_data(data, data) {
            Ok(s) => s,
            Err(err) => unreachable!("{}", err),
        }
    }

    /// Create a new sample with the given left and right channel
    /// data. Both the left and right channels must have the same
    /// amount of samples.
    pub fn with_stereo_data(left: &[f32], right: &[f32]) -> Result<Sample> {
        if left.len() != right.len() {
            return Err(anyhow!(
                "expected equal length for left and right channels but got length {} and {}",
                left.len(),
                right.len()
            ));
        }
        Ok(Sample {
            buffer: Arc::new(Vec::from_iter(left.iter().chain(right.iter()).copied())),
        })
    }

    /// Load an audio sample from a wave file.
    ///
    /// TODO: Provide sample rate  conversion.
    pub fn with_wave_file(path: impl AsRef<Path>) -> Result<Sample> {
        let path = path.as_ref();
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        let maybe_data: Result<Vec<_>, _> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect(),
            hound::SampleFormat::Int => reader
                .samples::<i16>()
                .map(|r| r.map(|v| v as f32 / i16::MAX as f32))
                .collect(),
        };
        let data =
            maybe_data.map_err(|err| anyhow!("failed to decode wave file {path:?}: {}", err))?;
        match spec.channels {
            0 => Err(anyhow!("wave file {path:?} had 0 channels")),
            1 => Ok(Sample::with_mono_data(&data)),
            2 => {
                let data = deinterleave_duplex(&data);
                let (l, r) = data.split_at(data.len() / 2);
                Sample::with_stereo_data(l, r)
            }
            n => Err(anyhow!(
                "expected 1 or 2 channels but got {n} from {path:?}"
            )),
        }
    }

    /// Iterate through all the samples in both the left and right
    /// channels.
    pub fn iter_samples(&self) -> SampleIter {
        SampleIter {
            sample: self.clone(),
            left_idx: 0,
            right_idx: self.buffer.len() / 2,
        }
    }
}

/// An iterator over a `Sample`.
#[derive(Clone, Debug, Default)]
pub struct SampleIter {
    /// The sample to iterate over.
    sample: Sample,
    /// The index within the sample for the left channel.
    left_idx: usize,
    /// The indes within the sample for the right channel.
    right_idx: usize,
}

impl SampleIter {
    /// Reset iteration to the start of the sample.
    pub fn reset(&mut self) {
        self.left_idx = 0;
        self.right_idx = self.sample.buffer.len() / 2;
    }

    /// End the iteration. Future calls to `next` will produce `None`.
    pub fn end(&mut self) {
        self.left_idx = self.sample.buffer.len();
        self.right_idx = self.sample.buffer.len();
    }
}

impl Iterator for SampleIter {
    type Item = (f32, f32);

    /// Get the next left and right audio elements.
    fn next(&mut self) -> Option<(f32, f32)> {
        let ret = match [
            self.sample.buffer.get(self.left_idx),
            self.sample.buffer.get(self.right_idx),
        ] {
            [Some(a), Some(b)] => Some((*a, *b)),
            _ => None,
        };
        self.left_idx += 1;
        self.right_idx += 1;
        ret
    }
}

/// Turns a stream of interleaved audio (2 channels only) into two
/// separate segments.
/// Example:
///   `[a1 b1 a2 b2 a3 b3] -> [a1 a2 a3 b1 b2 b3]`
fn deinterleave_duplex(data: &[f32]) -> Vec<f32> {
    let mut ret = vec![0.0; data.len()];
    for (idx, v) in data.iter().enumerate() {
        if idx % 2 == 0 {
            ret[idx / 2] = *v;
        } else {
            ret[data.len() / 2 + idx / 2] = *v;
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_with_mono_data_produces_same_data_for_both_channels() {
        let sample = Sample::with_mono_data(&[1.0, 2.0, 4.0, 8.0]);
        assert_eq!(
            sample.iter_samples().collect::<Vec<_>>(),
            vec![(1.0, 1.0), (2.0, 2.0), (4.0, 4.0), (8.0, 8.0)]
        );
    }

    #[test]
    fn test_sample_stereo_data_called_with_different_sizes_produces_error() {
        assert!(Sample::with_stereo_data(&[1.0], &[1.0, 2.0]).is_err());
    }

    #[test]
    fn test_sample_load_with_wave_file_that_does_not_exist_produces_error() {
        assert!(Sample::with_wave_file("/does/not/exist").is_err());
    }

    #[test]
    fn test_sample_load_with_wave_file_loads_sample() {
        let sample = Sample::with_wave_file("assets/LoFi-drum-loop.wav").unwrap();
        assert_eq!(sample.iter_samples().count(), 264600);
        assert_eq!(
            sample.iter_samples().take(2).collect::<Vec<_>>(),
            vec![(0.014374218, 0.014374218), (0.016052736, 0.016052736)],
        );
    }

    #[test]
    fn test_sample_iterate_over_samples_iterates_over_samples() {
        let sample = Sample::with_stereo_data(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]).unwrap();
        let got: Vec<_> = sample.iter_samples().collect();
        assert_eq!(got, vec![(1.0, 4.0), (2.0, 5.0), (3.0, 6.0)]);
    }

    #[test]
    fn test_sample_empty_sample_iteration_produces_no_values() {
        let sample = Sample::with_stereo_data(&[], &[]).unwrap();
        let got: Vec<_> = sample.iter_samples().collect();
        assert_eq!(got, vec![]);
    }

    #[test]
    fn test_sample_reset_sets_iterator_to_start() {
        let mut sample_iter = Sample::with_mono_data(&[1.0, 2.0]).iter_samples();

        assert_eq!(sample_iter.next(), Some((1.0, 1.0)));
        assert_eq!(sample_iter.next(), Some((2.0, 2.0)));
        assert_eq!(sample_iter.next(), None);

        sample_iter.reset();
        assert_eq!(sample_iter.next(), Some((1.0, 1.0)));
    }

    #[test]
    fn test_sample_end_ends_iteration() {
        let mut sample_iter = Sample::with_mono_data(&[1.0, 2.0]).iter_samples();
        sample_iter.end();
        assert_eq!(sample_iter.next(), None);
    }
}
