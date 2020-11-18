
use crate::command::{Command, CommandTrait, CommandSharedState};
use crate::config::config_dir;

use std::collections::HashMap;
use std::process::Child;
use std::thread;
use std::sync::{Arc, Mutex};
use std::io::Read as _;
use std::mem;

use run_script::{ScriptOptions, run_script, spawn_script};


#[derive(Clone, PartialEq)]
pub struct NoneCommand;
#[derive(Clone, PartialEq)]
pub struct LiteralCommand(pub String);
#[derive(Clone, PartialEq)]
pub struct MultiCommand(pub Vec<Command>);
#[derive(Clone, PartialEq)]
pub struct ShellCommand(pub String);
#[derive(Clone, PartialEq)]
pub struct PipeCommand(pub String);

#[derive(Default)]
struct PipeCommandProcess {
    child: Option<Child>,
    thread: Option<thread::JoinHandle<()>>,
    last_line: String,
    current_line: Arc<Mutex<String>>
}

#[derive(Default)]
struct PipeCommandSingleton {
    processes: HashMap<String, PipeCommandProcess>
}


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
        let mut options = ScriptOptions::new();
        options.working_directory = Some(config_dir());

        let (code, output, error) = run_script!(self.0, options)
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

impl CommandTrait for PipeCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        state.get::<PipeCommandSingleton>(0).process(&self.0).0
    }
    fn updated(&self, state: &mut CommandSharedState) -> bool {
        state.get::<PipeCommandSingleton>(0).process(&self.0).1
    }
}

impl PipeCommandSingleton {
    fn process(&mut self, cmd: &String) -> (String, bool) {
        self.processes.entry(cmd.clone()).or_default().update(cmd)
    }
}

impl PipeCommandProcess {

    fn start(&mut self, cmd: &String) {
        let mut options = ScriptOptions::new();
        options.working_directory = Some(config_dir());

        // Kill the child process to ensure the old thread will end
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
        self.child = match spawn_script!(cmd, options) {
            Ok(x) => Some(x),
            Err(e) => {eprintln!("Failed to spawn command {}: {}", cmd, e); return}
        };
        

        if let Some(output) = self.child.as_mut().unwrap().stdout.take() {
            
            // End the old thread safely
            if let Some(thread) = mem::replace(&mut self.thread, None) {
                let _ = thread.join();
            }

            let current_line = Arc::clone(&self.current_line);

            // Spawn a new thread for updating last line
            self.thread = Some(thread::spawn(move || {
                let mut buf = vec!(0u8; 1024);
                let mut c = 0;
                
                for i in output.bytes() {
                    match i {
                        Ok(b'\n') => {
                            *current_line.lock().unwrap() = String::from_utf8_lossy(&buf[0..c]).into_owned();
                            c = 0; continue;
                        }
                        Ok(x) => {
                            buf[c] = x;
                        }
                        Err(_) => {break}
                    }

                    c+=1;
                    c%=1024;
                }
            }));
        }
        else {
            eprintln!("Warning: process {} doesn't provide an stdout", cmd);
        }
    }

    fn update(&mut self, cmd: &String) -> (String, bool) {
        let restart = if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(None) => false, // Still running
                Ok(Some(status)) => {eprintln!("Process {} ended with status {}", cmd, status); true},
                Err(error) => {eprintln!("Error while checking process {}: {}", cmd, error); true}
            }
        }
        else {true};

        if restart {
            self.start(cmd)
        }

        let current = self.current_line.lock().unwrap();
        let updated = if self.last_line != *current {
            self.last_line = current.clone();
            true
        }
        else {false};

        (self.last_line.clone(), updated)
    }
   
}
