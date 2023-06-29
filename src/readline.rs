use log::warn;

/// Reads input from the command line and translates it into commands.
pub struct Readline {
    history_path: std::path::PathBuf,
    editor: rustyline::Editor<AutoComplete, rustyline::history::FileHistory>,
}

/// A command to execute.
#[derive(Copy, Clone, Debug)]
pub enum Command {
    /// List all plugins.
    ListPlugins,
    /// Create a new track with the given plugin index.
    AddTrack(usize),
    /// Instantiate a plugin to a track.
    AddPlugin { track: usize, plugin: usize },
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

/// The shell auto complete implementation.
struct AutoComplete {}

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
        editor.set_helper(Some(AutoComplete {}));
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

impl rustyline::Helper for AutoComplete {}

impl rustyline::completion::Completer for AutoComplete {
    type Candidate = String;

    /// Return the completion candidates given a line.
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let words = line.trim().split(" ");
        let word_count = words.clone().count();
        if pos != line.len() || word_count != 1 {
            return Ok((0, Vec::with_capacity(0)));
        }
        let word = words.last().unwrap_or_default();
        let cmds = ["add_plugin ", "add_track ", "exit", "help", "list_plugins"];
        let candidates: Vec<String> = cmds
            .into_iter()
            .filter(|c| c.starts_with(word))
            .map(String::from)
            .collect();
        Ok((0, candidates))
    }
}

impl rustyline::hint::Hinter for AutoComplete {
    type Hint = String;
}

impl rustyline::highlight::Highlighter for AutoComplete {}

impl rustyline::validate::Validator for AutoComplete {}

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
            "add_plugin" => {
                let mut numbers = parts.map(|s| -> Result<usize, _> { s.parse() });
                let track = numbers.next();
                let plugin = numbers.next();
                match (track, plugin) {
                    (None, _) => Err(Error::NotEnoughArgumentsForCommand {
                        command: "add_plugin",
                        expected_arguments: 2,
                        actual_arguments: 0,
                    }),
                    (_, None) => Err(Error::NotEnoughArgumentsForCommand {
                        command: "add_plugin",
                        expected_arguments: 2,
                        actual_arguments: 1,
                    }),
                    (Some(Err(err)), _) => Err(Error::FailedToParseInteger(err)),
                    (_, Some(Err(err))) => Err(Error::FailedToParseInteger(err)),
                    (Some(Ok(track)), Some(Ok(plugin))) => Ok(Command::AddPlugin { track, plugin }),
                }
            }
            "exit" => Ok(Command::Exit),
            cmd => Err(Error::UnknownCommand(cmd.to_string())),
        }
    }

    /// The help string.
    pub fn help_str() -> &'static str {
        r#"Commands:
    list_plugins                - List all available plugins.
    add_track <plugin>          - Add a track with the given plugin.
    add_plugin <track> <plugin> - Add to the track the given plugin.
    help                        - Print the help menu.
    exit                        - Exit the program."#
    }
}
