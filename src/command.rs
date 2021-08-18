
use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::hash::{Hasher, Hash};
use std::collections::hash_map::DefaultHasher;

use serde_yaml::{Value, from_value};
use serde::Deserialize;
use dyn_clone::DynClone;

mod common;
mod sysinfo;
mod alsa;
pub mod state;

// A general trait for commands, concrete implementations are in command/ directory
pub trait CommandTrait: 'static + Any + DynClone {
    fn execute(&self,  state: &mut CommandSharedState) -> String;
    fn updated(&self, _state: &mut CommandSharedState) -> bool {
        false
    }
}
dyn_clone::clone_trait_object!(CommandTrait);

// Provides a way to create/access shared structures of commands
pub struct CommandSharedState {
    parts: HashMap<(TypeId, u64), Box<dyn Any>>
}

// A command container used in other program structs
#[derive(Clone)]
pub struct Command {
    cmd: Box<dyn CommandTrait>,
    id: u64
}

// Wrapper for YAML object
#[derive(Debug, Deserialize)]
struct CommandObject {
    r#type: Option<String>,
    core: Option<usize>,
    network: Option<String>,
    mountpoint: Option<String>,
    card: Option<String>,
    volume: Option<String>,
    state_machine: Option<String>,
    state: Option<String>,
    traverse: Option<i32>
}


// This trait is only used when comparing "current" properties in order to redraw the widget.
// Doing it "the right way" results in a lot of redundant redraws, heavily increasing CPU usage.
impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl CommandTrait for Command {
    fn execute(&self, state: &mut CommandSharedState) -> String {self.cmd.execute(state)}
    fn updated(&self, state: &mut CommandSharedState) -> bool {self.cmd.updated(state)}
}

impl CommandSharedState {
    pub fn new() -> Self { Self {parts: HashMap::new()}}

    // Access a singleton or create one if it doesn't exist
    pub fn get<T: Any + Default>(&mut self, id: u64) -> &mut T {
        self.parts.entry((TypeId::of::<T>(), id))
            .or_insert_with( || Box::new(T::default()) )
            .downcast_mut::<T>().unwrap()
    }
}


impl From<Value> for Command {
    fn from(val: Value) -> Self {
        // Coming up with a better implementation is left as an exercise for the reader
        
        let mut hasher = DefaultHasher::new();
        format!("{:?}", val).hash(&mut hasher);
        Self { cmd: new_command(val), id: hasher.finish() }
    }
}

fn new_command(val: Value) -> Box<dyn CommandTrait> {
    match val {
        Value::String(s) => {
            let mut rem = s.chars().skip_while(|x| x.is_whitespace());
            match rem.next() {
                Some(c) => match c {
                    '#' => Box::new(common::LiteralCommand(rem.collect())),
                    '|' => Box::new(common::PipeCommand(rem.collect())),
                     _  => Box::new(common::ShellCommand(s))
                }
                None => Box::new(common::NoneCommand)
            }
        }
        // Note - child commands can have id 0, because they will never be compared
        Value::Sequence(v) => Box::new(common::MultiCommand(v.iter()
                        .map(|s| Command::from(s.to_owned()))
                        .collect())),
        Value::Mapping(obj) => {
            let object: CommandObject = from_value(Value::Mapping(obj)).unwrap();
            
            if let Some(t) = object.r#type {
                
                let words = t.split("_").collect::<Vec<_>>();
                
                match words.get(0) {
                    Some(&"cpu") => match words.get(1) {
                        Some(&"usage") => Box::new(sysinfo::CPUUsageCommand(object.core)),
                        Some(&"freq")  => Box::new(sysinfo::CPUFreqCommand(object.core)),
                        _ => panic!("Unknown command type {}", t)
                    }
                    Some(&"mem") | Some(&"swap") | Some(&"disk") => {
                        let ty = match words.get(0) {
                            Some(&"mem") => sysinfo::MemoryInfoType::RAM,
                            Some(&"swap") => sysinfo::MemoryInfoType::Swap,
                            Some(&"disk") => sysinfo::MemoryInfoType::Disk(object.mountpoint),
                            _ => panic!("Unknown command type {}", t)
                        };
                        let val = match words.get(1) {
                            Some(&"usage") => sysinfo::MemoryInfoValue::Usage,
                            Some(&"percent") => sysinfo::MemoryInfoValue::Percent,
                            Some(&"total") => sysinfo::MemoryInfoValue::Total,
                            Some(&"free") => sysinfo::MemoryInfoValue::Free,
                            _ => panic!("Unknown command type {}", t)
                        };
                        Box::new(sysinfo::MemoryInfoCommand {ty, val})
                    }
                    Some(&"net") => {
                        let ty = match words.get(1) {
                            Some(&"upload") => sysinfo::NetInfoType::Upload,
                            Some(&"download") => sysinfo::NetInfoType::Download,
                            _ => panic!("Unknown command type {}", t)
                        };
                        let val = match words.get(2) {
                            Some(&"bits") => sysinfo::NetInfoValue::Bits,
                            Some(&"bytes") => sysinfo::NetInfoValue::Bytes,
                            Some(&"packets") => sysinfo::NetInfoValue::Packets,
                            Some(&"errors") => sysinfo::NetInfoValue::Errors,
                            _ => panic!("Unknown command type {}", t)
                        };
                        let time = match words.get(3) {
                            Some(&"since") => sysinfo::NetInfoTime::Since,
                            Some(&"total") => sysinfo::NetInfoTime::Total,
                            None => sysinfo::NetInfoTime::PerSecond,
                            _ => panic!("Unknown command type {}", t)
                        };
                        Box::new(sysinfo::NetInfoCommand {ty,val,time, name: object.network})
                    }
                    Some(&"alsa") => {
                        if let Some(&"volume") = words.get(2) {
                            match words.get(1) {
                                Some(&"get") => Box::new(alsa::ALSAGetVolumeCommand(object.card)),
                                Some(&"set") => Box::new(alsa::ALSASetVolumeCommand(
                                        object.card,
                                        alsa::VolumeChange::new(object.volume.unwrap_or_default()))
                                    ),
                                _ => {panic!("Unknown command type {}", t)}
                            }
                        }
                        else {panic!("Unknown command type {}", t)}
                    }
                    Some(&"state") => {
                        let sm = object.state_machine.expect("'state_*' commands require 'state_machine' to be set");

                        match words.get(1) {
                            Some(&"next") => Box::new(state::NextStateCommand(sm, object.traverse.unwrap_or(1))),
                            Some(&"set")  => Box::new(state::SetStateCommand(sm, object.state.expect("'state_set' command requires 'state' to be set"))),
                            _ => {panic!("Unknown command type {}", t)}
                        }
                    }
                    _ => panic!("Unknown command type {}", t)
                }
            }
            else {
                panic!("'type' property of command must exist if it's an object (command: {:#?})", object);
            }
        }
        _ => panic!("'command' must be either a string, an object with a required value 'type' or an array of those")
    }
}

