pub mod moog_filter;
pub mod sawtooth;

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
