
use super::{Event, EventTrait, EventListener};
use crate::bar::Bar;


#[derive(Debug, Hash, Clone)]
pub struct DefaultEvent;

pub struct DefaultListener;


impl EventTrait for DefaultEvent {
    fn precedence(&self) -> u32 {1000}
    fn mouse_dependent(&self) -> bool {false}
    fn is_expose(&self) -> bool {false}
}

crate::impl_hashed_simple!(DefaultEvent);

impl EventListener for DefaultListener {
    
    fn reported_events(&self) -> &'static[&'static str] {
        const FILES_EVENTS: &'static[&'static str] = &[&"default"];
        FILES_EVENTS
    }

    fn event(&mut self, event: &String, settings: &String) -> Event {
        match &event[..] {
            "default" => Box::new(DefaultEvent),
            _ => panic!("Unknown event {}.{} (reported by DefaultListener)", event, settings)
        }
    }
    fn get(&mut self, _bar: &Bar, v: &mut Vec<Event>) {
        v.push(Box::new(DefaultEvent))
    }
}
