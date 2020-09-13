

use std::collections::HashMap;

use serde::Deserialize;

use crate::event::Event;
use crate::command::*;
use crate::window::Direction;
use crate::draw::Drawable;

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

/// A macro for super convenient bar/widget properties declaration
/// Definitions are at the bottom of props.rs
macro_rules! prop_struct {
    ($Props:ident, $PropsCurrent:ident, $ConfigProps:ident, 
     $( $name:ident : $type:ident from $rawtype:ident = $default:expr),*) => {
        pub struct $Props {
            $(pub $name: Prop<$type>,)*
        }

        #[derive(Clone, PartialEq)]
        pub struct $PropsCurrent {
            $(pub $name: $type,)*
        }

        #[derive(Deserialize, Clone, Debug)]
        pub struct $ConfigProps {
            $($name: Option<$rawtype>,)*
        }

        impl $Props {
            pub fn as_current(&self, e: &Vec<Event>, m: bool) -> $PropsCurrent {
                $PropsCurrent {
                    $($name: self.$name.get(e,m).clone(),)*
                }
            }
        }

        impl Default for $ConfigProps {
            fn default() -> Self {
                Self {
                    $($name: None,)*
                }
            }
        }

        impl $ConfigProps {

            #[allow(dead_code)]
            pub fn mix(&mut self, parent: &Self) -> &Self {
                $(self.$name = mix_options(&parent.$name, &self.$name);)*
                self
            }
        }

        impl $Props {
            pub fn from(config: &HashMap<(String, String), $ConfigProps>) -> Self {
                Self {
                    $($name: prop!(config, $name, $type, $default),)*
                }
            }
        }
    }
}

fn mix_options<T: Clone>(parent: &Option<T>, child: &Option<T>) -> Option<T> {
    match child {
        Some(x) => Some(x.clone()),
        None => match parent {Some(y) => Some(y.clone()), None => None}
    }
}

// Widget properties
prop_struct!(WidgetProps, WidgetPropsCurrent, BarConfigWidgetProps, 
             
    foreground:     Drawable from String = Drawable::from("#FFFFFF".to_string()),
    background:     Drawable from String = Drawable::from("#223333".to_string()),
    command:        Command from String = Command::None,
    border_factor:  f32 from f32 = 0.9,
    interval:       f32 from f32 = 5.0
);

// Bar properties
prop_struct!(BarProps, BarPropsCurrent, BarConfigProps, 

    alignment:  Direction from String = Direction::from("NW".to_string()),
    height:     u16 from u16 = 30
);

