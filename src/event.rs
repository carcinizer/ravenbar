
use crate::config::config_dir;
use crate::bar::Bar;

use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;
use x11rb::protocol::Event as XEvent;


#[derive(Debug, Hash, Clone)]
pub enum Event {
    Default,
    Expose,
    Hover,
    ButtonPress(Option<u8>),
    ButtonPressCont(Option<u8>),
    ButtonRelease(Option<u8>),
    ButtonReleaseCont(Option<u8>),
    FileChanged(std::path::PathBuf),
}

pub trait HashedSimple {
    fn hashed(&self) -> u64;
}

pub trait EventTrait: HashedSimple + DynClone + Debug {
    fn precedence(&self) -> u32;
    fn mouse_dependent(&self) -> bool;
}

dyn_clone::clone_trait_object!(EventTrait);

pub trait EventListener {
    fn reported_events(&self) -> &'static[&'static str];
    fn event(&self, event: &String, settings: &String) -> Event;
    fn get(&self, bar: &Bar) -> Vec<Event>;
}

struct EventListeners {
    listeners: Vec<Box<dyn EventListener>>
}

/*impl<T> Hash for T 
where T: EventTrait {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.hashed().hash(h);
    }
}

impl<T> PartialEq for T 
where T: EventTrait {
    fn eq(&self, other: &Self) -> bool {
        self.hashed() == other.hashed()
    }
}*/

impl<T: Hash> HashedSimple for T {
    fn hashed(&self) -> u64 {
        let mut a = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut a);
        a.finish()
    }
}

impl Event {
    pub fn from(event: &String, settings: &String) -> Self {
        match &event[..] {
            "default" => Self::Default,
            "on_hover" => Self::Hover,
            "on_press" => Self::ButtonPress(mouse_button(settings)),
            "on_press_cont" => Self::ButtonPressCont(mouse_button(settings)),
            "on_release" => Self::ButtonRelease(mouse_button(settings)),
            "on_release_cont" => Self::ButtonReleaseCont(mouse_button(settings)),
            "on_file_changed" => Self::FileChanged(config_dir().join(settings)),
            _ => {panic!("Invalid event {}.{}", event, settings)}
        }
    }

    pub fn events_from(ev: XEvent) -> Vec<Self> {
        match ev {
            XEvent::Expose(_) => vec![Self::Expose],
            XEvent::ButtonPress(x) => vec![Self::ButtonPress(None), Self::ButtonPress(Some(x.detail))],
            XEvent::ButtonRelease(x) => vec![Self::ButtonRelease(None), Self::ButtonRelease(Some(x.detail))],
            _ => { eprintln!("Unknown event: {:?}, reverting to default", ev); vec![Self::Default]}
        }
    }

}

impl EventTrait for Event {
    fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPress(b) => 101 + add_precedence(b),
            Self::ButtonRelease(b) => 101 + add_precedence(b),
            Self::ButtonPressCont(b) => 102 + add_precedence(b),
            Self::ButtonReleaseCont(b) => 102 + add_precedence(b),
            Self::FileChanged(_) => 150,
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
}

impl EventListeners {
}

fn mouse_button(s: &String) -> Option<u8> {
    match &s[..] {
        "" => None,
        "left" => Some(1), 
        "middle" => Some(2), 
        "right" => Some(3), 
        "scroll_up" => Some(4), 
        "scroll_down" => Some(5), 
        _ => Some(u8::from_str_radix(s, 10)
                  .expect("Mouse button must be either a number or one of: (left, middle, right, scroll_up, scroll_down)"))
    }
}

fn add_precedence(b: &Option<u8>) -> u32 {
    match b {
        Some(_) => 0,
        None => 5
    }
}
