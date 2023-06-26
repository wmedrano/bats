pub struct Readline {
    editor: rustyline::DefaultEditor,
}

impl Readline {
    pub fn new() -> Result<Self, rustyline::error::ReadlineError> {
        Ok(Readline {
            editor: rustyline::DefaultEditor::new()?,
        })
    }

    pub fn readline(&mut self) -> Result<Command, Error> {
        match self.editor.readline(">> ") {
            Ok(line) => Command::parse(&line),
            Err(rustyline::error::ReadlineError::Interrupted) => Ok(Command::Exit),
            Err(rustyline::error::ReadlineError::Eof) => Ok(Command::Help),
            Err(err) => Err(Error::ReadlineError(err)),
        }
    }
}

pub enum Command {
    ListPlugins,
    SetPlugin(usize),
    Help,
    Nothing,
    Exit,
}

#[derive(Debug)]
pub enum Error {
    UnknownCommand(String),
    NotEnoughArgumentsForCommand {
        command: &'static str,
        expected_arguments: usize,
        actual_arguments: usize,
    },
    FailedToParseInteger(std::num::ParseIntError),
    ReadlineError(rustyline::error::ReadlineError),
}

impl Command {
    fn parse(line: &str) -> Result<Command, Error> {
        let mut parts = line.trim().split(" ");
        match parts.next().unwrap_or("") {
            "help" => Ok(Command::Help),
            "" => Ok(Command::Nothing),
            "list_plugins" => Ok(Command::ListPlugins),
            "set_plugin" => match parts.next().map(|s| -> Result<usize, _> { s.parse() }) {
                None => Err(Error::NotEnoughArgumentsForCommand {
                    command: "set_plugin",
                    expected_arguments: 1,
                    actual_arguments: 0,
                }),
                Some(Err(err)) => Err(Error::FailedToParseInteger(err)),
                Some(Ok(idx)) => Ok(Command::SetPlugin(idx)),
            },
            "exit" => Ok(Command::Exit),
            cmd => Err(Error::UnknownCommand(cmd.to_string())),
        }
    }

    pub fn help_str() -> &'static str {
        r#"Commands:
    list_plugins    - List all available plugins.
    set_plugin <id> - Set the plugin.
    help            - Print the help menu.
    exit            - Exit the program."#
    }
}
