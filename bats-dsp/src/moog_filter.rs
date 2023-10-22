/// A classic Moog low pass filter.
///
/// Credit: Implementation is derived from
/// https://github.com/ddiakopoulos/MoogLadders/blob/master/src/MusicDSPModel.h.
#[derive(Copy, Clone, Debug)]
pub struct MoogFilter {
    cutoff: f32,
    resonance: f32,
    stage: [f32; 4],
    delay: [f32; 4],
    p: f32,
    k: f32,
    t1: f32,
    t2: f32,
}

impl MoogFilter {
    /// The default cutoff frequency.
    const DEFAULT_CUTOFF_FREQUENCY: f32 = 8000.0;

    /// The default resonance.
    const DEFAULT_RESONANCE: f32 = 0.1;

    /// Create a new `MoogFilter`.
    pub fn new(sample_rate: f32) -> MoogFilter {
        let mut f = MoogFilter {
            cutoff: 0.0,
            resonance: 0.0,
            stage: [0.0; 4],
            delay: [0.0; 4],
            p: 0.0,
            k: 0.0,
            t1: 0.0,
            t2: 0.0,
        };
        f.set_cutoff(
            sample_rate,
            MoogFilter::DEFAULT_CUTOFF_FREQUENCY,
            MoogFilter::DEFAULT_RESONANCE,
        );
        f
    }

    /// Set the cutoff frequency and resonance.
    pub fn set_cutoff(&mut self, sample_rate: f32, cutoff_frequency: f32, resonance: f32) {
        self.cutoff = 2.0 * cutoff_frequency / sample_rate;
        self.p = self.cutoff * (1.8 - 0.8 * self.cutoff);
        self.k = 2.0 * (self.cutoff * std::f32::consts::PI * 0.5).sin() - 1.0;
        self.t1 = (1.0 - self.p) * 1.386249;
        self.t2 = 12.0 + self.t1 * self.t1;
        self.resonance = resonance * (self.t2 + 6.0 * self.t1) / (self.t2 - 6.0 * self.t1);
    }

    /// Process the next sample.
    pub fn process(&mut self, sample: f32) -> f32 {
        let x = sample - self.resonance * self.stage[3];

        // Four cascaded one-pole filters (bilinear transform).
        self.stage[0] =
            (x * self.p + self.delay[0] * self.p - self.k * self.stage[0]).clamp(-1.0, 1.0);
        self.stage[1] = self.stage[0] * self.p + self.delay[1] * self.p
            - self.k * self.stage[1].clamp(-1.0, 1.0);
        self.stage[2] = self.stage[1] * self.p + self.delay[2] * self.p
            - self.k * self.stage[2].clamp(-1.0, 1.0);
        self.stage[3] = self.stage[2] * self.p + self.delay[3] * self.p
            - self.k * self.stage[3].clamp(-1.0, 1.0);

        // Clipping band-limited sigmoid
        self.stage[3] -= (self.stage[3] * self.stage[3] * self.stage[3]) / 6.0;
        self.stage[3] = self.stage[3].clamp(-1.0, 1.0);

        self.delay[0] = x;
        self.delay[1] = self.stage[0];
        self.delay[2] = self.stage[1];
        self.delay[3] = self.stage[2];

        self.stage[3]
    }
}
