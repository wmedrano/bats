use log::warn;

/// Reads input from the command line and translates it into commands.
pub struct Readline {
    history_path: std::path::PathBuf,
    editor: rustyline::Editor<(), rustyline::history::FileHistory>,
}

impl Readline {
    /// Create a new `Readline`.
    pub fn new() -> Result<Self, rustyline::error::ReadlineError> {
        let history_path = shellexpand::full("~/.config/simian-sonic.history").unwrap();
        let config = rustyline::Config::builder()
            .max_history_size(100)?
            .auto_add_history(true)
            .build();
        let history = rustyline::history::FileHistory::new();
        let mut editor = rustyline::Editor::with_history(config, history)?;
        if let Err(err) = editor.load_history(history_path.as_ref()) {
            warn!(
                "Could not load history from {}: {}",
                history_path.as_ref(),
                err
            );
            let _ = std::fs::write(history_path.as_ref(), "");
        }
        Ok(Readline {
            history_path: history_path.as_ref().into(),
            editor,
        })
    }

    /// Read the next command.
    pub fn readline(&mut self) -> Result<Command, Error> {
        match self.editor.readline(">> ") {
            Ok(line) => {
                if let Err(err) = self.editor.save_history(&self.history_path) {
                    warn!("Could not save history: {}", err);
                }
                Command::parse(&line)
            }
            Err(rustyline::error::ReadlineError::Interrupted) => Ok(Command::Exit),
            Err(rustyline::error::ReadlineError::Eof) => Ok(Command::Help),
            Err(err) => Err(Error::ReadlineError(err)),
        }
    }
}

/// A command to execute.
#[derive(Copy, Clone, Debug)]
pub enum Command {
    /// List all plugins.
    ListPlugins,
    /// Create a new track with the given plugin index.
    AddTrack(usize),
    /// Print help.
    Help,
    /// Do nothing.
    Nothing,
    /// Exit the program.
    Exit,
}

/// Errors that occur when reading commands.
#[derive(Debug)]
pub enum Error {
    /// The command is not known.
    UnknownCommand(String),
    /// Not enough arguments for the specified command.
    NotEnoughArgumentsForCommand {
        command: &'static str,
        expected_arguments: usize,
        actual_arguments: usize,
    },
    /// Failed to parse an integer.
    FailedToParseInteger(std::num::ParseIntError),
    /// An error in `rustyline`.
    ReadlineError(rustyline::error::ReadlineError),
}

impl Command {
    /// Parse a command from a line.
    fn parse(line: &str) -> Result<Command, Error> {
        let mut parts = line.trim().split(" ");
        match parts.next().unwrap_or("") {
            "help" => Ok(Command::Help),
            "" => Ok(Command::Nothing),
            "list_plugins" => Ok(Command::ListPlugins),
            "add_track" => match parts.next().map(|s| -> Result<usize, _> { s.parse() }) {
                None => Err(Error::NotEnoughArgumentsForCommand {
                    command: "add_track",
                    expected_arguments: 1,
                    actual_arguments: 0,
                }),
                Some(Err(err)) => Err(Error::FailedToParseInteger(err)),
                Some(Ok(idx)) => Ok(Command::AddTrack(idx)),
            },
            "exit" => Ok(Command::Exit),
            cmd => Err(Error::UnknownCommand(cmd.to_string())),
        }
    }

    /// The help string.
    pub fn help_str() -> &'static str {
        r#"Commands:
    list_plugins    - List all available plugins.
    add_track  <id> - Add a track with the given plugin.
    help            - Print the help menu.
    exit            - Exit the program."#
    }
}
