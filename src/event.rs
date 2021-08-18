
use crate::bar::Bar;
use crate::command::CommandSharedState;

use std::fmt::Debug;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;

mod files;
mod window;
mod default;
mod state;


pub type Event = Box<dyn EventTrait>;

pub trait HashedSimple {
    fn hashed(&self) -> u64;
}

pub trait EventTrait: HashedSimple + DynClone + Debug {
    fn precedence(&self) -> u32;
    fn mouse_dependent(&self) -> bool;
    fn is_expose(&self) -> bool;
}

dyn_clone::clone_trait_object!(EventTrait);

pub trait EventListener {

    /// List event names that this listener support
    fn reported_events(&self) -> &'static[&'static str];
    
    /// Create event object from event description and optionally remember its settings
    fn event(&mut self, cmd: &mut CommandSharedState, event: &String, settings: &String) -> Event;
    
    /// Add events to the event vector
    fn get(&mut self, bar: &Bar, v: &mut Vec<Event>);
}

pub struct EventListeners {
    listeners: Vec<Box<dyn EventListener>>,
    event_map: HashMap<String, usize>
}

impl Hash for Event {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.hashed().hash(h);
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.hashed() == other.hashed()
    }
}

impl Eq for Event {}

#[macro_export]
macro_rules! impl_hashed_simple(($type:ty, $id:expr) => {
    use crate::event::HashedSimple as HS;
    use std::hash::Hash;
    use std::hash::Hasher;

    impl HS for $type { 
        fn hashed(&self) -> u64 {
            let mut a = std::collections::hash_map::DefaultHasher::new();
            self.hash(&mut a);
            $id.hash(&mut a);
            a.finish()
        }
    }
});


impl Default for Event {
    fn default() -> Self {
        Box::new(default::DefaultEvent)
    }
}


impl EventListeners {
    pub fn new() -> Self {
        
        let listeners: Vec<Box<dyn EventListener>> = vec![
            Box::new(files::FilesListener::new()),
            Box::new(window::WindowListener::new()),
            Box::new(state::StateListener::new()),
            Box::new(default::DefaultListener)
        ];

        let event_map = listeners.iter()
            .enumerate()
            .flat_map(|(c,x)| x.reported_events().iter().map(move |i| (c,i)))
            .map(|(c, x)| (x.to_string(), c))
            .collect();

        Self {listeners, event_map}
    }

    pub fn event(&mut self, cmd: &mut CommandSharedState, event: &String, settings: &String) -> Event {
        let e = || panic!("Invalid event {}.{}: No listener found for this event", event, settings);

        self.listeners[*self.event_map.get(event).unwrap_or_else(e)]
            .event(cmd, event, settings)
    }

    pub fn get(&mut self, bar: &Bar) -> Vec<Event> {

        let mut v = Vec::with_capacity(40);

        for i in self.listeners.iter_mut() {
            i.get(bar, &mut v);
        }

        v.sort_by_key(|x| x.precedence());
        v
    }
}

