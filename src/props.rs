

use std::collections::HashMap;

use crate::event::Event;
use crate::command::*;
use crate::window::{Drawable, Direction};

pub struct Prop<T> {
    pub map: HashMap<Event, T>
}


impl<T> Prop<T> {
    pub fn get(&self, events: &Vec<Event>, mouse_inside: bool) -> &T {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(x) = self.map.get(i) {
                return x;
            }
        }
        panic!("Somewhere something doesn't have any events!");
    }

    pub fn get_event<'a>(&self, events: &Vec<Event>, mouse_inside: bool) -> Event {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(_) = self.map.get(i) {
                return i.clone();
            }
        }
        panic!("Somewhere something doesn't have any events!");
    }
}

#[macro_export]
macro_rules! prop {
    ($var:expr, $member:ident, $type:ident, $default:expr) => {{
        
        use std::collections::HashMap;
        
        let mut map = HashMap::new();
        map.insert(Event::Default, $default);
        
        for ((k,s),v) in $var.iter() {
            if let Some(x) = &v.$member {
                map.insert(Event::from(k, s), $type::from(x.clone()));
            }
        }
        Prop {map}
    }}
}

pub struct WidgetProps {
    pub foreground: Prop<Drawable>,
    pub background: Prop<Drawable>,
    pub command: Prop<Command>,
    pub border_factor: Prop<f32>,
    pub interval: Prop<f32>
}

#[derive(Clone, PartialEq)]
pub struct WidgetPropsCurrent {
    pub foreground: Drawable,
    pub background: Drawable,
    pub command: Command,
    pub border_factor: f32,
    pub interval: f32
}

pub struct BarProps {
    pub alignment: Prop<Direction>,
    pub height: Prop<u16>
}

#[derive(Clone, PartialEq)]
pub struct BarPropsCurrent {
    pub alignment: Direction,
    pub height: u16
}

impl WidgetProps {
    pub fn as_current(&self, e: &Vec<Event>, m: bool) -> WidgetPropsCurrent {
        WidgetPropsCurrent {
            foreground: self.foreground.get(e,m).clone(),
            background: self.background.get(e,m).clone(),
            command: self.command.get(e,m).clone(),
            border_factor: self.border_factor.get(e,m).clone(),
            interval: self.interval.get(e,m).clone()}
    }
}

impl BarProps {
    pub fn as_current(&self, e: &Vec<Event>, m: bool) -> BarPropsCurrent {
        BarPropsCurrent {
            alignment: self.alignment.get(e,m).clone(),
            height: self.height.get(e,m).clone()
        }
    }
}
