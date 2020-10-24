
use crate::command::{Command, CommandTrait, CommandSharedState};
use crate::config::config_dir;


#[derive(Clone, PartialEq)]
pub struct NoneCommand;
#[derive(Clone, PartialEq)]
pub struct LiteralCommand(pub String);
#[derive(Clone, PartialEq)]
pub struct MultiCommand(pub Vec<Command>);
#[derive(Clone, PartialEq)]
pub struct ShellCommand(pub String);


impl CommandTrait for NoneCommand {
    fn execute(&self, _state: &mut CommandSharedState) -> String {
        String::new()
    }
}

impl CommandTrait for LiteralCommand {
    fn execute(&self, _state: &mut CommandSharedState) -> String {
        self.0.clone()
    }
}

impl CommandTrait for MultiCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        self.0.iter().map(|x| x.execute(state)).collect::<Vec<_>>().join("")
    }
}

impl CommandTrait for ShellCommand {
    fn execute(&self, _state: &mut CommandSharedState) -> String {
        let mut options = run_script::ScriptOptions::new();
        options.working_directory = Some(config_dir());

        let (code, output, error) = run_script::run_script!(self.0, options)
            .expect("Failed to run shell script");

        if code != 0 {
            eprintln!("WARNING: '{}' returned {}", self.0, code);
        }
        if !error.chars()
            .filter(|x| !x.is_control())
            .eq(std::iter::empty()) {

            eprintln!("WARNING: '{}' wrote to stderr:", self.0);
            eprintln!("{}", error);
        }
        output
    }
}
