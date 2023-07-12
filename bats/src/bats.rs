use std::borrow::{Borrow, BorrowMut};

use log::error;

use crate::{ipc::Ipc, track::Track};

/// Handles audio processing.
pub struct Bats {
    /// The plugin instance to run or `None` if no plugin should be running.
    pub tracks: Vec<Track>,

    /// A temporary `LV2AtomSequence` to use for processing. The object is persisted to avoid
    /// allocating memory.
    atom_sequence_input: livi::event::LV2AtomSequence,
    /// The `urid` for the LV2 midi atom.
    midi_urid: u32,
    /// A channel to receive functions to execute.
    remote_fns: crossbeam_channel::Receiver<crate::ipc::RawFn>,
    /// A buffer that can be used to store temporary data.
    buffer: Vec<f32>,
}

impl Bats {
    /// The maximum amount of tracks.
    pub const TRACKS_CAPACITY: usize = 64;
    pub const PLUGIN_INSTANCE_CAPACITY: usize = 32;

    /// Create a new `ProcessHandler`.
    pub fn new(features: &livi::Features) -> Self {
        let atom_sequence_input = livi::event::LV2AtomSequence::new(features, 4096);
        let midi_urid = features.midi_urid();
        let (_, remote_fns) = crossbeam_channel::bounded(1);
        Bats {
            tracks: Vec::with_capacity(Self::TRACKS_CAPACITY),
            atom_sequence_input,
            midi_urid,
            remote_fns,
            buffer: vec![0f32; features.max_block_length() * 32],
        }
    }

    /// Reset the remote executor and return it.
    ///
    /// Any previously set executor will no longer be responsive.
    pub fn reset_remote_executor(&mut self, queue_size: usize) -> Ipc {
        let (tx, rx) = crossbeam_channel::bounded(queue_size);
        self.remote_fns = rx;
        Ipc::new(tx)
    }

    /// Process data and write the results to `audio_out`.
    pub fn process<'a>(
        &'a mut self,
        frames: usize,
        midi_in: impl Iterator<Item = (i64, &'a [u8])>,
        audio_out: [&'a mut [f32]; 2],
    ) {
        // All the scenarios are OK.
        let _ = self.handle_remote_fns();
        Self::load_midi_events(&mut self.atom_sequence_input, midi_in, self.midi_urid);
        Self::process_tracks(
            frames,
            &mut self.tracks,
            &self.atom_sequence_input,
            audio_out,
            &mut self.buffer,
        );
    }

    /// Run all remote functions that have been queued.
    fn handle_remote_fns(&mut self) -> Result<(), crossbeam_channel::TryRecvError> {
        let mut f = self.remote_fns.try_recv()?;
        loop {
            (f.f)(self);
            f = self.remote_fns.try_recv()?;
        }
    }

    fn load_midi_events<'a>(
        dst: &mut livi::event::LV2AtomSequence,
        src: impl Iterator<Item = (i64, &'a [u8])>,
        midi_urid: u32,
    ) {
        dst.clear();
        for (frame, data) in src {
            dst.push_midi_event::<4>(frame, midi_urid, data).unwrap();
        }
    }

    /// Process all tracks and write the results to out.
    fn process_tracks(
        frames: usize,
        tracks: &mut [Track],
        atom_sequence: &livi::event::LV2AtomSequence,
        mut audio_out: [&mut [f32]; 2],
        buffer: &mut [f32],
    ) {
        for slice in audio_out.iter_mut() {
            clear(slice);
        }

        for track in tracks.iter_mut() {
            if !track.enabled {
                continue;
            }
            let mut buffer_chunks = buffer.chunks_exact_mut(frames);
            let mut tmp_in = [buffer_chunks.next().unwrap(), buffer_chunks.next().unwrap()];
            let mut tmp_out = [buffer_chunks.next().unwrap(), buffer_chunks.next().unwrap()];
            for b in tmp_out.iter_mut() {
                clear(b);
            }
            for plugin_instance in track.plugin_instances.iter_mut() {
                std::mem::swap(&mut tmp_in, &mut tmp_out);
                let port_counts = plugin_instance.instance.port_counts();
                let ports = livi::EmptyPortConnections::new()
                    .with_audio_inputs(
                        tmp_in
                            .iter()
                            .map(Borrow::borrow)
                            .take(port_counts.audio_inputs),
                    )
                    .with_audio_outputs(
                        tmp_out
                            .iter_mut()
                            .map(BorrowMut::borrow_mut)
                            .take(port_counts.audio_outputs),
                    )
                    .with_atom_sequence_inputs(
                        std::iter::once(atom_sequence).take(port_counts.atom_sequence_inputs),
                    );
                let res = unsafe { plugin_instance.instance.run(frames, ports) };
                if let Err(err) = res {
                    track.enabled = false;
                    error!("{:?}", err);
                    error!(
                        "Disabling plugin {:?}.",
                        plugin_instance.instance.raw().instance().uri()
                    );
                    continue;
                }
            }
            if track.enabled {
                for (dst, src) in audio_out.iter_mut().zip(tmp_out.iter()) {
                    mix(dst, src, track.volume);
                }
            }
        }
    }
}

fn clear(a: &mut [f32]) {
    for v in a.iter_mut() {
        *v = 0f32;
    }
}

fn mix(dst: &mut [f32], src: &[f32], ratio: f32) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += *s * ratio;
    }
}
