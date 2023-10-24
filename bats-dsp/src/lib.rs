use std::fmt;

pub mod moog_filter;
pub mod sawtooth;

#[derive(Clone, PartialEq)]
pub struct Buffers {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl Buffers {
    pub fn new(len: usize) -> Buffers {
        Buffers {
            left: vec![0.0; len],
            right: vec![0.0; len],
        }
    }
}

impl fmt::Debug for Buffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffers")
            .field("length", &self.left.len())
            .field("left", &self.left.iter().take(4))
            .field("right", &self.right.iter().take(4))
            .finish()
    }
}

/// Contains the sample rate.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SampleRate {
    seconds_per_sample: f32,
}

impl SampleRate {
    /// Create a new sample rate.
    #[inline]
    pub fn new(sample_rate: f32) -> SampleRate {
        SampleRate {
            seconds_per_sample: sample_rate.recip(),
        }
    }

    /// Get the sample rate.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.seconds_per_sample.recip()
    }

    /// Get the number of seconds per sample.
    #[inline]
    pub fn seconds_per_sample(&self) -> f32 {
        self.seconds_per_sample
    }

    /// Get the frequency as a ratio of the sample rate.
    #[inline]
    pub fn normalized_frequency(&self, freq: f32) -> f32 {
        self.seconds_per_sample * freq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_frequency() {
        let sample_rate = SampleRate::new(44100.0);
        assert_eq!(sample_rate.normalized_frequency(22050.0), 0.5);
    }
}
