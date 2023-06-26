mod process_handler;

fn main() {
    let world_handle = std::thread::spawn(livi::World::new);
    let (client, _status) =
        jack::Client::new("simian-sonic", jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate() as f64;

    let world = world_handle.join().unwrap();
    let features = world.build_features(livi::FeaturesBuilder::default());
    let mut process_handler = process_handler::ProcessHandler::new(&client, &features).unwrap();
    let mutator = process_handler.reset_mutator();
    process_handler.connect(&client).unwrap();
    let active_client = client.activate_async((), process_handler).unwrap();

    let mut rl = rustyline::DefaultEditor::new().unwrap();
    let mut user_requested_exit = false;
    Command::print_help();
    while !user_requested_exit {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => match Command::parse(&line) {
                Command::ListPlugins => {
                    for (idx, plugin) in world.iter_plugins().enumerate() {
                        println!("{}: {}", idx, plugin.name());
                    }
                }
                Command::SetPlugin(idx) => {
                    let plugin = world.iter_plugins().nth(idx).unwrap();
                    let plugin_instance =
                        unsafe { plugin.instantiate(features.clone(), sample_rate) }.unwrap();
                    mutator.mutate(move |ph| ph.plugin_instance = Some(plugin_instance));
                }
                Command::Help => Command::print_help(),
                Command::Exit => user_requested_exit = true,
                Command::UnknownCommand(err) => println!("Unknown command: {}", err),
            },
            Err(rustyline::error::ReadlineError::Interrupted) => user_requested_exit = true,
            Err(rustyline::error::ReadlineError::Eof) => {}
            Err(err) => panic!("Readline error: {:?}", err),
        }
    }

    println!("Exiting...");
    active_client.deactivate().unwrap();
}

enum Command {
    ListPlugins,
    SetPlugin(usize),
    Help,
    Exit,
    UnknownCommand(String),
}

impl Command {
    pub fn parse(line: &str) -> Command {
        let mut parts = line.split(" ");
        match parts.next().unwrap_or("") {
            "list_plugins" => Command::ListPlugins,
            "set_plugin" => Command::SetPlugin(parts.next().unwrap().parse().unwrap()),
            "help" | "" => Command::Help,
            "exit" => Command::Exit,
            cmd => Command::UnknownCommand(cmd.to_string()),
        }
    }

    pub fn print_help() {
        println!(
            r#"Commands:
    list_plugins    - List all available plugins.
    set_plugin <id> - Set the plugin.
    help            - Print the help menu.
    exit            - Exit the program."#
        );
    }
}
