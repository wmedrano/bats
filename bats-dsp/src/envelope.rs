use crate::sample_rate::SampleRate;

/// The parameters for an envelope.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EnvelopeParams {
    /// The amount of amp to add in each attack phase sample.
    attack_delta: f32,
    /// The amount of amp to add in each decay phase sample. This should be negative.
    decay_delta: f32,
    /// The amount of amp to add in each release sample. This should be negative.
    release_delta: f32,
    /// The amp level of the sustain phase.
    sustain_amp: f32,
}

impl Default for EnvelopeParams {
    /// Create a default instance of an envelope. The default sustains an amp at 1.0 immediately and
    /// cuts off immediately after release.
    fn default() -> EnvelopeParams {
        EnvelopeParams {
            attack_delta: 1.0,
            decay_delta: 1.0,
            release_delta: 1.0,
            sustain_amp: 1.0,
        }
    }
}

impl EnvelopeParams {
    /// Create a new envelope params object.
    pub fn new(
        sample_rate: SampleRate,
        attack_seconds: f32,
        decay_seconds: f32,
        sustain_amp: f32,
        release_seconds: f32,
    ) -> EnvelopeParams {
        let attack_frames = sample_rate.sample_rate() * attack_seconds;
        let attack_delta = 1.0 / attack_frames;
        let decay_frames = sample_rate.sample_rate() * decay_seconds;
        let decay_delta = -(1.0 - sustain_amp) / decay_frames;
        let release_frames = sample_rate.sample_rate() * release_seconds;
        let release_delta = -sustain_amp / release_frames;
        EnvelopeParams {
            attack_delta,
            decay_delta,
            release_delta,
            sustain_amp,
        }
    }
}

/// Handles envelope logic.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Envelope {
    /// The current stage in the envelope.
    stage: Stage,
    /// The current amp.
    amp: f32,
}

/// The stage of the envelope.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
enum Stage {
    /// The attack phase.
    #[default]
    Attack,
    /// The decay phase.
    Decay,
    /// The sustain phase.
    Sustain,
    /// The release phase.
    Release,
    /// The final phase which means that the envelope is done and produces no (value of 0.0) signal.
    Done,
}

impl Envelope {
    /// Create a new envelope.
    pub fn new() -> Envelope {
        Envelope {
            stage: Stage::Attack,
            amp: 0.0,
        }
    }

    /// Get the next sample in the envelope.
    pub fn next_sample(&mut self, params: &EnvelopeParams) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.amp += params.attack_delta;
                if self.amp >= 1.0 {
                    self.amp = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.amp += params.decay_delta;
                if self.amp <= params.sustain_amp {
                    self.amp = params.sustain_amp;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {}
            Stage::Release => {
                self.amp += params.release_delta;
                if self.amp < 0.0 {
                    self.amp = 0.0;
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => {}
        }
        self.amp
    }

    /// Release the envelope and begin the release phase.
    pub fn release(&mut self, params: &EnvelopeParams) {
        self.amp = self.amp.min(params.sustain_amp);
        self.stage = Stage::Release;
    }

    /// Returns true if the envelope is still active.
    pub fn is_active(&self) -> bool {
        self.stage != Stage::Done
    }
}
