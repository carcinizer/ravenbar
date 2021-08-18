
use super::{Event, EventTrait, EventListener};
use crate::bar::Bar;
use crate::command::CommandSharedState;
use crate::command::state::StateSingleton;


#[derive(Debug, Clone, Hash)]
struct StateIs(String, i32);

pub struct StateListener {
    subscriptions: Vec<(String, i32)>
}


impl EventTrait for StateIs {
    fn precedence(&self) -> u32 {300}
    fn mouse_dependent(&self) -> bool {false}
    fn is_expose(&self) -> bool {false}
}

crate::impl_hashed_simple!(StateIs, 100030);


impl EventListener for StateListener {
    
    fn reported_events(&self) -> &'static[&'static str] {
        const STATE_EVENTS: &'static[&'static str] = &[&"on_state"];
        STATE_EVENTS
    }

    fn event(&mut self, cmd: &mut CommandSharedState, event: &String, settings: &String) -> Event {

        let states = cmd.get::<StateSingleton>(0);

        match &event[..] {
            "on_state" => {
                
                let mut sett = settings.splitn(2, '=');
                let machine = sett.next().expect(&format!("Failed to get state machine for event {}.{}", event, settings)).to_string();
                let state   = sett.next().expect(&format!("Failed to get state for event {}.{}", event, settings)).to_string();

                let sid = states.get_state_id(&machine, &state);

                self.subscriptions.push((machine.clone(), sid));

                Box::new(StateIs(machine, sid))
            },
            _ => panic!("Unknown event {}.{} (reported by StateListener)", event, settings)
        }
    }

    fn get(&mut self, bar: &Bar, v: &mut Vec<Event>) {
        let mut cmdstate = bar.get_cmd_state();
        let states = cmdstate.get::<StateSingleton>(0);

        for (m,s) in self.subscriptions.iter() {
            if states.get(m) == *s {
                v.push(Box::new(StateIs(m.clone(), *s)));
            }
        }
    }
}

impl StateListener {
    pub fn new() -> Self {
        Self {subscriptions: vec!()}
    }
}
