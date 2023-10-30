use crate::sample_rate::SampleRate;

/// A sawtooth wave.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Sawtooth {
    amplitude: f32,
    amplitude_per_sample: f32,
}

impl Sawtooth {
    /// Create a new Sawtooth wave.
    #[inline]
    pub fn new(sample_rate: SampleRate, frequency: f32) -> Sawtooth {
        let amplitude_per_cycle = 2.0;
        let cycles_per_second = frequency;
        let amplitude_per_sample =
            amplitude_per_cycle * cycles_per_second * sample_rate.seconds_per_sample();
        Sawtooth {
            amplitude: 0.0,
            amplitude_per_sample,
        }
    }

    /// Set the frequency for the Sawtooth wave.
    #[inline]
    pub fn set_frequency(&mut self, sample_rate: SampleRate, frequency: f32) {
        let amplitude_per_cycle = 2.0;
        let cycles_per_second = frequency;
        let amplitude_per_sample =
            amplitude_per_cycle * cycles_per_second * sample_rate.seconds_per_sample();
        self.amplitude_per_sample = amplitude_per_sample;
    }

    /// Get the next sample in the sawtooth wave.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        self.amplitude += self.amplitude_per_sample;
        if self.amplitude > 1.0 {
            self.amplitude -= 2.0;
        }
        self.amplitude
    }
}
