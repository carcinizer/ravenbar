

use std::collections::HashMap;

use serde::Deserialize;

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


#[derive(Deserialize, Clone, Debug)]
pub struct BarConfigProps {
    pub alignment: Option<String>,
    pub height: Option<u16>
}


#[derive(Deserialize, Clone, Debug)]
pub struct BarConfigWidgetProps {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub command: Option<String>,
    pub border_factor: Option<f32>,
    pub interval: Option<f32>
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

impl BarConfigWidgetProps {
    pub fn new() -> Self {
        Self { 
            background: None,
            foreground: None,
            command: None,
            border_factor: None,
            interval: None
        }
    }

    pub fn mix(&mut self, parent: &Self) -> &mut Self {
        self.background = mix_options(&parent.background, &self.background);
        self.foreground = mix_options(&parent.foreground, &self.foreground);
        self.command    = mix_options(&parent.command,    &self.command);
        self
    }
}

impl BarConfigProps {
    pub fn new() -> Self {
        Self {
            alignment: None,
            height: None
        }
    }

    pub fn mix(&mut self, parent: &Self) -> &mut Self {
        self.alignment = mix_options(&parent.alignment, &self.alignment);
        self.height    = mix_options(&parent.height,    &self.height);
        self
    }
}

impl std::default::Default for BarConfigProps {
    fn default() -> Self { Self::new() }
}

impl std::default::Default for BarConfigWidgetProps {
    fn default() -> Self { Self::new() }
}

fn mix_options<T: Clone>(parent: &Option<T>, child: &Option<T>) -> Option<T> {
    match child {
        Some(x) => Some(x.clone()),
        None => match parent {Some(y) => Some(y.clone()), None => None}
    }
}
