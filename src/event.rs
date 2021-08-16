
use crate::bar::Bar;

use std::fmt::Debug;
use std::collections::HashMap;

use dyn_clone::DynClone;
use x11rb::protocol::Event as XEvent;

mod files;

#[derive(Debug, Hash, Clone)]
pub enum LegacyEvent {
    Default,
    Expose,
    Hover,
    ButtonPress(Option<u8>),
    ButtonPressCont(Option<u8>),
    ButtonRelease(Option<u8>),
    ButtonReleaseCont(Option<u8>),
}

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
    fn event(&mut self, event: &String, settings: &String) -> Event;
    
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
macro_rules! impl_hashed_simple(($type:ty) => {
    use crate::event::HashedSimple as HS;
    use std::hash::Hash;
    use std::hash::Hasher;

    impl HS for $type { 
        fn hashed(&self) -> u64 {
            let mut a = std::collections::hash_map::DefaultHasher::new();
            self.hash(&mut a);
            a.finish()
        }
    }
});

impl_hashed_simple!(LegacyEvent);

impl Default for Event {
    fn default() -> Self {
        Box::new(LegacyEvent::Default)
    }
}

impl From<(String, String)> for Event {
    fn from((event, settings): (String, String)) -> Event {
        Box::new( match &event[..] {
            "default" => LegacyEvent::Default,
            "on_hover" => LegacyEvent::Hover,
            "on_press" => LegacyEvent::ButtonPress(mouse_button(settings)),
            "on_press_cont" => LegacyEvent::ButtonPressCont(mouse_button(settings)),
            "on_release" => LegacyEvent::ButtonRelease(mouse_button(settings)),
            "on_release_cont" => LegacyEvent::ButtonReleaseCont(mouse_button(settings)),
            _ => {panic!("Invalid event {}.{}", event, settings)}
        })
    }
}

pub fn events_from(ev: XEvent) -> Vec<Event> {
    match ev {
        XEvent::Expose(_) => vec![Box::new(LegacyEvent::Expose)],
        XEvent::ButtonPress(x) => vec![Box::new(LegacyEvent::ButtonPress(None)), Box::new(LegacyEvent::ButtonPress(Some(x.detail)))],
        XEvent::ButtonRelease(x) => vec![Box::new(LegacyEvent::ButtonRelease(None)), Box::new(LegacyEvent::ButtonRelease(Some(x.detail)))],
        _ => { eprintln!("Unknown event: {:?}, reverting to default", ev); vec![Box::new(LegacyEvent::Default)]}
    }
}

impl EventTrait for LegacyEvent {
    fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPress(b) => 101 + add_precedence(b),
            Self::ButtonRelease(b) => 101 + add_precedence(b),
            Self::ButtonPressCont(b) => 102 + add_precedence(b),
            Self::ButtonReleaseCont(b) => 102 + add_precedence(b),
            Self::Expose => 160,
            Self::Hover => 200,
            Self::Default => 1000
        }
    }

    fn mouse_dependent(&self) -> bool {
        match self {
            Self::Hover => true,
            Self::ButtonPress(_) => true,
            Self::ButtonRelease(_) => true,
            Self::ButtonPressCont(_) => true,
            Self::ButtonReleaseCont(_) => true,
            _ => false
        }
    }

    fn is_expose(&self) -> bool {
        match self {
            Self::Expose => true,
            _ => false
        }
    }
}

impl EventListeners {
    pub fn new() -> Self {
        
        let listeners: Vec<Box<dyn EventListener>> = vec![
            Box::new(files::FilesListener::new())
        ];

        let event_map = listeners.iter()
            .enumerate()
            .flat_map(|(c,x)| x.reported_events().iter().map(move |i| (c,i)))
            .map(|(c, x)| (x.to_string(), c))
            .collect();

        Self {listeners, event_map}
    }

    pub fn event(&mut self, event: &String, settings: &String) -> Event {
        let e = || panic!("Invalid event {}.{}: No listener found for this event", event, settings);

        self.listeners[*self.event_map.get(event).unwrap_or_else(e)]
            .event(event, settings)
    }

    pub fn get(&mut self, bar: &Bar) -> Vec<Event> {

        let mut v = Vec::with_capacity(10);

        for i in self.listeners.iter_mut() {
            i.get(bar, &mut v);
        }
        v
    }
}

fn mouse_button(s: String) -> Option<u8> {
    match &s[..] {
        "" => None,
        "left" => Some(1), 
        "middle" => Some(2), 
        "right" => Some(3), 
        "scroll_up" => Some(4), 
        "scroll_down" => Some(5), 
        _ => Some(u8::from_str_radix(&s, 10)
                  .expect("Mouse button must be either a number or one of: (left, middle, right, scroll_up, scroll_down)"))
    }
}

fn add_precedence(b: &Option<u8>) -> u32 {
    match b {
        Some(_) => 0,
        None => 5
    }
}
