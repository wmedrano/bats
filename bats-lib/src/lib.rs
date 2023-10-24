use bats_dsp::SampleRate;
use metronome::Metronome;
use plugin::{toof::Toof, BatsInstrument};
use position::Position;

pub mod metronome;
pub mod plugin;
pub mod position;

/// Handles all processing.
#[derive(Debug)]
pub struct Bats {
    /// The metronome.
    pub metronome: Metronome,
    /// The volume for the metronome.
    pub metronome_volume: f32,
    /// The positions for each sample.
    ///
    /// Note: The first entry in the slice represents the previous
    /// position.
    transport: Vec<Position>,
    /// The active plugins.
    pub plugins: Vec<PluginWithBuffer>,
    /// The sample rate.
    pub sample_rate: SampleRate,
    pub buffer_size: usize,
}

/// An plugin with output buffers.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginWithBuffer {
    /// The id for this plugin instance.
    pub id: u32,
    /// The plugin.
    pub plugin: Toof,
    /// The left audio output.
    pub left: Vec<f32>,
    /// The right audio output.
    pub right: Vec<f32>,
}

impl Bats {
    /// Create a new `Bats` object.
    pub fn new(sample_rate: SampleRate, buffer_size: usize) -> Bats {
        Bats {
            metronome: Metronome::new(sample_rate, 120.0),
            metronome_volume: 0.0,
            transport: Vec::with_capacity(buffer_size + 1),
            plugins: Vec::with_capacity(16),
            sample_rate,
            buffer_size,
        }
    }

    /// Add a new plugin.
    pub fn add_plugin(&mut self, plugin: PluginWithBuffer) {
        self.plugins.push(plugin);
    }

    /// Process midi data and output audio.
    pub fn process(
        &mut self,
        midi: &[(u32, wmidi::MidiMessage<'static>)],
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let sample_count = left.len().min(right.len());
        process_metronome(
            sample_count,
            &mut self.metronome,
            self.metronome_volume,
            left,
            right,
            &mut self.transport,
        );
        for plugin in self.plugins.iter_mut() {
            plugin
                .plugin
                .process_batch(midi, &mut plugin.left, &mut plugin.right);
            mix(left, &plugin.left, 0.25);
            mix(right, &plugin.right, 0.25);
        }
    }

    /// Iterate over all plugins.
    pub fn iter_plugins(&self) -> impl Iterator<Item = &Toof> {
        self.plugins.iter().map(|p| &p.plugin)
    }
}

/// Process the metronome data. This produces the metronome sound and
/// updates the `transport` variable.
fn process_metronome(
    sample_count: usize,
    metronome: &mut Metronome,
    metronome_volume: f32,
    left: &mut [f32],
    right: &mut [f32],
    transport: &mut Vec<Position>,
) {
    left.fill(0.0);
    right.fill(0.0);
    let mut previous = match transport.pop() {
        Some(p) => p,
        None => {
            left[0] = metronome_volume;
            right[0] = metronome_volume;
            Position::default()
        }
    };
    transport.clear();
    transport.push(previous);
    for i in 0..sample_count {
        let next = metronome.next_position();
        if previous.beat() != next.beat() {
            left[i] = metronome_volume;
            right[i] = metronome_volume;
        }
        transport.push(next);
        previous = next;
    }
}

fn mix(dst: &mut [f32], src: &[f32], volume: f32) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += volume * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_input_produces_silence() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let mut b = Bats::new(SampleRate::new(44100.0), left.len());
        b.process(&[], &mut left, &mut right);
        assert_eq!(left, [0.0, 0.0, 0.0]);
        assert_eq!(right, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn no_input_with_metronome_produces_metronome() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let mut b = Bats::new(SampleRate::new(44100.0), left.len());
        b.metronome_volume = 0.8;
        b.process(&[], &mut left, &mut right);
        assert_eq!(left, [0.8, 0.0, 0.0]);
        assert_eq!(right, [0.8, 0.0, 0.0]);
    }

    #[test]
    fn metronome_ticks_regularly() {
        let mut left = vec![0.0; 44100];
        let mut right = vec![0.0; 44100];
        let mut bats = Bats::new(SampleRate::new(44100.0), 44100);
        bats.metronome_volume = 0.8;
        bats.metronome.set_bpm(SampleRate::new(44100.0), 120.0);
        bats.process(&[], &mut left, &mut right);
        // At 120 BPM, it should tick twice in a second.
        assert_eq!(left.iter().filter(|v| 0.0 != **v).count(), 2);
        assert_eq!(right.iter().filter(|v| 0.0 != **v).count(), 2);
    }
}
