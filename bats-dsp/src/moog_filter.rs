use crate::sample_rate::SampleRate;

/// A classic Moog low pass filter.
///
/// Credit: Implementation is derived from
/// https://github.com/ddiakopoulos/MoogLadders/blob/master/src/MusicDSPModel.h.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MoogFilter {
    r: f32,
    stage: [f32; 4],
    delay: [f32; 4],
    p: f32,
    k: f32,
}

impl MoogFilter {
    /// The default cutoff frequency.
    pub const DEFAULT_FREQUENCY_CUTOFF: f32 = 8000.0;

    /// The default resonance.
    pub const DEFAULT_RESONANCE: f32 = 0.1;

    /// Create a new `MoogFilter`.
    pub fn new(sample_rate: SampleRate) -> MoogFilter {
        let mut f = MoogFilter {
            r: 0.0,
            stage: [0.0; 4],
            delay: [0.0; 4],
            p: 0.0,
            k: 0.0,
        };
        f.set_cutoff(
            sample_rate,
            MoogFilter::DEFAULT_FREQUENCY_CUTOFF,
            MoogFilter::DEFAULT_RESONANCE,
        );
        f
    }

    /// Set the cutoff frequency and resonance.
    pub fn set_cutoff(&mut self, sample_rate: SampleRate, cutoff_frequency: f32, resonance: f32) {
        let cutoff = 2.0 * cutoff_frequency * sample_rate.seconds_per_sample();
        self.p = cutoff * (1.8 - 0.8 * cutoff);
        self.k = 2.0 * (cutoff * std::f32::consts::PI * 0.5).sin() - 1.0;
        let t1 = (1.0 - self.p) * 1.386249;
        let t2 = 12.0 + t1 * t1;
        self.r = resonance * (t2 + 6.0 * t1) / (t2 - 6.0 * t1);
    }

    /// Process the next sample.
    pub fn process(&mut self, sample: f32) -> f32 {
        let x = sample - self.r * self.stage[3];

        // Four cascaded one-pole filters (bilinear transform).
        self.stage[0] = x * self.p + self.delay[0] * self.p - self.k * self.stage[0];
        self.stage[0] = self.stage[0].clamp(-1.0, 1.0);
        self.stage[1] = self.stage[0] * self.p + self.delay[1] * self.p - self.k * self.stage[1];
        self.stage[2] = self.stage[1] * self.p + self.delay[2] * self.p - self.k * self.stage[2];
        self.stage[3] = self.stage[2] * self.p + self.delay[3] * self.p - self.k * self.stage[3];

        // Clipping band-limited sigmoid
        self.stage[3] -= (self.stage[3] * self.stage[3] * self.stage[3]) / 6.0;
        self.stage[3] = self.stage[3].clamp(-1.0, 1.0);

        self.delay[0] = x;
        self.delay[1] = self.stage[0];
        self.delay[2] = self.stage[1];
        self.delay[3] = self.stage[2];

        self.stage[3]
    }

    /// Filter apply filtering in `dst` in place.
    pub fn process_batch(&mut self, dst: &mut [f32]) {
        for out in dst.iter_mut() {
            *out = self.process(*out);
        }
    }
}
