
use crate::command::{CommandTrait, CommandSharedState};
use crate::utils::LogType;

use std::collections::HashMap;


#[derive(Clone, PartialEq)]
pub struct SetStateCommand(pub String, pub String);

#[derive(Clone, PartialEq)]
pub struct NextStateCommand(pub String, pub i32);

struct StateMachine {
    states: HashMap<String, i32>,
    current: i32
}

#[derive(Default)]
pub struct StateSingleton {
    states: HashMap<String, StateMachine>
}


impl From<&Vec<String>> for StateMachine {
    fn from(states: &Vec<String>) -> Self {
        let states = states.iter().enumerate().map(|(c,i)| (i.clone(), c as i32)).collect();
        Self {states, current: 0}
    }
}

impl StateSingleton {
    pub fn initialize(&mut self, states: &HashMap<String, Vec<String>>) {
        self.states = states.iter().map(|(k,v)| (k.clone(), StateMachine::from(v))).collect();
    }


    pub fn get_state_id(&mut self, machine: &String, state: &String) -> i32 {
        *self.states.get_mut(machine).expect(&format!("No state machine named '{}'", machine))
             .states.get_mut(state).expect(&format!("No state '{}' in '{}'", state, machine))
    }

    fn next(&mut self, machine: &String, traverse: i32) {
        self.states.get_mut(machine).or_else(|| {crate::log!(LogType::Warning, "No state machine named '{}'", machine); None})
            .and_then(|x| {x.current = (x.current + traverse) % x.states.len() as i32; Some(())});
    }

    fn set(&mut self, machine: &String, state: &String) {
        self.states.get_mut(machine).or_else(|| {crate::log!(LogType::Warning, "No state machine named '{}'", machine); None})
            .and_then(|x| {
                x.current = *x.states.get(state)
                    .unwrap_or_else(|| {
                        crate::log!(LogType::Warning, "No state machine named '{}'", machine);
                        &x.current
            }); Some(())});
    }

    pub fn get(&self, machine: &String) -> i32 {
        // "get" here should never fail after singleton + listener initialization
        self.states.get(machine).and_then(|x| Some(x.current)).unwrap_or(-1)
    }
}

impl CommandTrait for SetStateCommand {
    fn execute(&self,  state: &mut CommandSharedState) -> String {
        state.get::<StateSingleton>(0).set(&self.0, &self.1);
        String::new()
    }
}

impl CommandTrait for NextStateCommand {
    fn execute(&self,  state: &mut CommandSharedState) -> String {
        state.get::<StateSingleton>(0).next(&self.0, self.1);
        String::new()
    }
}
