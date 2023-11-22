use serde::{Deserialize, Serialize};

use crate::sample_rate::SampleRate;

/// The parameters for an envelope.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeParams {
    /// The amount of amp to add in each attack phase sample.
    attack_delta: f32,
    /// The amount of amp to add in each decay phase sample. This should be negative.
    decay_delta: f32,
    /// The amount of amp to add in each release sample. This should be negative.
    release_delta: f32,
    /// The amp level of the sustain phase.
    sustain_amp: f32,
    /// The decay in seconds. Required in cases where recomputation is needed and decay is not
    /// computable.
    decay_seconds: f32,
}

impl Default for EnvelopeParams {
    /// Create a default instance of an envelope. The default sustains an amp at 1.0 immediately and
    /// cuts off immediately after release.
    fn default() -> EnvelopeParams {
        EnvelopeParams {
            attack_delta: 1.0,
            decay_delta: -1.0,
            release_delta: -1.0,
            sustain_amp: 1.0,
            decay_seconds: 0.0,
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
        let mut p = EnvelopeParams::default();
        p.set_attack(sample_rate, attack_seconds);
        p.set_decay(sample_rate, decay_seconds);
        p.set_sustain(sample_rate, sustain_amp);
        p.set_release(sample_rate, release_seconds);
        p
    }

    /// Get the attack value in seconds.
    pub fn attack(&self, sample_rate: SampleRate) -> f32 {
        let attack_samples = 1.0 / self.attack_delta;
        attack_samples * sample_rate.seconds_per_sample()
    }

    /// Set the attack value.
    pub fn set_attack(&mut self, sample_rate: SampleRate, attack_seconds: f32) {
        debug_assert!(attack_seconds >= 0.0);
        if attack_seconds == 0.0 {
            self.attack_delta = 1.0;
        } else {
            let attack_frames = sample_rate.sample_rate() * attack_seconds;
            self.attack_delta = 1.0 / attack_frames;
        }
    }

    /// Get the decay value.
    pub fn decay(&self, _sample_rate: SampleRate) -> f32 {
        self.decay_seconds
    }

    /// Set the decay.
    pub fn set_decay(&mut self, sample_rate: SampleRate, decay_seconds: f32) {
        debug_assert!(decay_seconds >= 0.0);
        self.decay_seconds = decay_seconds;
        if decay_seconds == 0.0 || self.sustain_amp == 1.0 {
            self.decay_delta = -1.0;
        } else {
            let decay_frames = sample_rate.sample_rate() * decay_seconds;
            self.decay_delta = (self.sustain_amp - 1.0) / decay_frames;
        }
        debug_assert!(self.decay_delta < 0.0);
    }

    /// Returns the sustain of this [`EnvelopeParams`].
    pub fn sustain(&self) -> f32 {
        self.sustain_amp
    }

    /// Sets the sustain of this [`EnvelopeParams`].
    pub fn set_sustain(&mut self, sample_rate: SampleRate, sustain_amp: f32) {
        debug_assert!(
            (0.0..=1.0).contains(&sustain_amp),
            "0.0 <= {sustain_amp} <= 1.0"
        );
        self.sustain_amp = sustain_amp;
        self.set_decay(sample_rate, self.decay_seconds);
    }

    /// Get the release value in seconds.
    pub fn release(&self, sample_rate: SampleRate) -> f32 {
        let release_frames = -self.sustain_amp / self.release_delta;
        release_frames * sample_rate.seconds_per_sample()
    }

    /// Sets the release of this [`EnvelopeParams`].
    pub fn set_release(&mut self, sample_rate: SampleRate, release_seconds: f32) {
        debug_assert!(release_seconds >= 0.0);
        if release_seconds == 0.0 {
            self.release_delta = -1.0;
        } else {
            let release_frames = sample_rate.sample_rate() * release_seconds;
            self.release_delta = -self.sustain_amp / release_frames;
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
#[derive(Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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
    /// An envelope that has been completely released and is no longer active.
    pub const INACTIVE: Envelope = Envelope {
        stage: Stage::Done,
        amp: 0.0,
    };

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

    /// Iterate through many samples.
    pub fn iter_samples<'a>(
        &'a mut self,
        params: &'a EnvelopeParams,
        count: usize,
    ) -> impl 'a + ExactSizeIterator + Iterator<Item = f32> {
        (0..count).map(|_| self.next_sample(params))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_active_until_release() {
        let params = EnvelopeParams::default();
        let base = Envelope::new();
        {
            let mut active = base;
            for _ in active.iter_samples(&params, 1000) {}
            assert!(active.is_active(), "{:?}", active);
        }
        {
            let mut released = base.clone();
            released.release(&params);
            assert!(released.is_active(), "{:?}", released);
            for _ in released.iter_samples(&params, 1000) {}
            assert!(!released.is_active(), "{:?}", released);
        }
    }

    #[test]
    fn default_envelope() {
        let params = EnvelopeParams::default();
        let sample_rate = SampleRate::new(64.0);
        // The minimum attack is at least 1 frame.
        assert_eq!(params.attack(sample_rate), 1.0 / 64.0);
        assert_eq!(params.decay(sample_rate), 0.0);
        assert_eq!(params.sustain(), 1.0);
        // The minimum release is at least 1 frame.
        assert_eq!(params.release(sample_rate), 1.0 / 64.0);
    }

    #[test]
    fn inactive_envelope_produces_no_signal() {
        let params = EnvelopeParams::default();
        let mut env = Envelope::INACTIVE;
        let output: Vec<_> = env.iter_samples(&params, 10000).collect();
        assert_eq!(output, vec![0.0; 10000]);
    }

    #[test]
    fn large_attack_takes_long_time_to_reach_1() {
        let sample_rate = SampleRate::new(4.0);
        let params = EnvelopeParams::new(sample_rate, 2.0, 0.0, 1.0, 0.0);
        let mut env = Envelope::new();
        let output: Vec<_> = env.iter_samples(&params, 10).collect();
        assert_eq!(
            output,
            vec![
                1.0 / 8.0,
                2.0 / 8.0,
                3.0 / 8.0,
                4.0 / 8.0,
                5.0 / 8.0,
                6.0 / 8.0,
                7.0 / 8.0,
                1.0,
                1.0,
                1.0
            ],
            "params={params:?}"
        );
    }

    #[test]
    fn get_and_set_params_produces_consistent_values() {
        let mut params = EnvelopeParams::default();
        let sample_rate = SampleRate::new(64.0);
        params.set_attack(sample_rate, 5.0);
        params.set_decay(sample_rate, 6.0);
        params.set_sustain(sample_rate, 0.7);
        params.set_release(sample_rate, 0.8);
        assert_eq!(params.attack(sample_rate), 5.0);
        assert_eq!(params.decay(sample_rate), 6.0);
        assert_eq!(params.sustain(), 0.7);
        assert_eq!(params.release(sample_rate), 0.8);
    }

    #[test]
    fn zero_second_durations_are_ok() {
        let mut params = EnvelopeParams::default();
        let sample_rate = SampleRate::new(64.0);
        params.set_attack(sample_rate, 0.0);
        params.set_decay(sample_rate, 0.0);
        params.set_sustain(sample_rate, 0.0);
        params.set_release(sample_rate, 0.0);
        // At least 1 frame is required.
        assert_eq!(params.attack(sample_rate), 1.0 / 64.0);
        assert_eq!(params.decay(sample_rate), 0.0);
        assert_eq!(params.sustain(), 0.0);
        assert_eq!(params.release(sample_rate), 0.0);
    }

    #[test]
    #[should_panic]
    fn bad_attack_panics() {
        EnvelopeParams::default().set_attack(SampleRate::new(44100.0), -1.0);
    }

    #[test]
    #[should_panic]
    fn bad_decay_panics() {
        EnvelopeParams::default().set_decay(SampleRate::new(44100.0), -1.0);
    }

    #[test]
    #[should_panic]
    fn bad_sustain_panics() {
        EnvelopeParams::default().set_sustain(SampleRate::new(44100.0), -1.0);
    }

    #[test]
    #[should_panic]
    fn bad_release_panics() {
        EnvelopeParams::default().set_release(SampleRate::new(44100.0), -1.0);
    }
}
