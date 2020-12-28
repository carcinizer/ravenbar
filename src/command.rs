
use std::collections::HashMap;
use std::any::{Any, TypeId};

use serde_json::{Value, from_value};
use serde::Deserialize;
use dyn_clone::DynClone;

mod common;
mod sysinfo;

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
    cmd: Box<dyn CommandTrait>
}

// Wrapper for JSON object
#[derive(Debug, Deserialize)]
struct CommandObject {
    r#type: Option<String>,
    core: Option<usize>,
    network: Option<String>,
    mountpoint: Option<String>
}


// This trait is only used when comparing "current" prop structs in order to redraw the widget.
// Doing it "the right way" results in a lot of redundant redraws, heavily increasing CPU usage.
impl PartialEq for Command {
    fn eq(&self, _other: &Self) -> bool {
        true
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
        Self { cmd: new_command(val) }
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
        Value::Array(v) => Box::new(common::MultiCommand(v.iter()
                        .map(|s| Command {cmd: new_command(s.to_owned())})
                        .collect())),
        Value::Object(obj) => {
            let object: CommandObject = from_value(Value::Object(obj)).unwrap();
            
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

