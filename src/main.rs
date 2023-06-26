fn main() {
    let world_handle = std::thread::spawn(livi::World::new);
    let (client, _status) =
        jack::Client::new("simian-sonic", jack::ClientOptions::NO_START_SERVER).unwrap();

    let world = world_handle.join().unwrap();
    let plugin = world
        .iter_plugins()
        .find(|p| p.uri() == "http://drobilla.net/plugins/mda/EPiano")
        .unwrap();
    let features = world.build_features(livi::FeaturesBuilder::default());
    let plugin_instance = unsafe { plugin.instantiate(features.clone(), 44100.0) }.unwrap();
    for (idx, plugin) in world.iter_plugins().enumerate() {
        println!("{}: {}", idx, plugin.name());
    }
    let process_handler = ProcessHandler::new(&client, plugin_instance, &features);
    let active_client = client.activate_async((), process_handler).unwrap();

    println!("Press RET to exit.");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();

    active_client.deactivate().unwrap();
}

struct ProcessHandler {
    plugin_instance: livi::Instance,
    audio_outputs: [jack::Port<jack::AudioOut>; 2],
    midi_input: jack::Port<jack::MidiIn>,
    atom_sequence_input: livi::event::LV2AtomSequence,
    midi_urid: u32,
}

impl ProcessHandler {
    pub fn new(
        c: &jack::Client,
        plugin_instance: livi::Instance,
        features: &livi::Features,
    ) -> Self {
        let audio_outputs = [
            c.register_port("out1", jack::AudioOut).unwrap(),
            c.register_port("out2", jack::AudioOut).unwrap(),
        ];
        let midi_input = c.register_port("midi", jack::MidiIn).unwrap();
        let atom_sequence_input = livi::event::LV2AtomSequence::new(features, 4096);
        let midi_urid = features.midi_urid();
        ProcessHandler {
            plugin_instance,
            audio_outputs,
            midi_input,
            atom_sequence_input,
            midi_urid,
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.atom_sequence_input.clear();
        for midi in self.midi_input.iter(ps) {
            self.atom_sequence_input
                .push_midi_event::<4>(midi.time as i64, self.midi_urid, midi.bytes)
                .unwrap();
        }
        let ports = livi::EmptyPortConnections::new()
            .with_audio_outputs(self.audio_outputs.iter_mut().map(|p| p.as_mut_slice(ps)))
            .with_atom_sequence_inputs(std::iter::once(&self.atom_sequence_input));
        unsafe { self.plugin_instance.run(ps.n_frames() as usize, ports) }.unwrap();
        jack::Control::Continue
    }
}
