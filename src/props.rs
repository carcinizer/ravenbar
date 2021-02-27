
use crate::event::Event;
use crate::command::*;
use crate::window::Direction;
use crate::draw::Drawable;

use std::collections::HashMap;

use serde_json::Value;
use serde::Deserialize;


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

    pub fn get_event(&self, events: &Vec<Event>, mouse_inside: bool) -> Event {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(_) = self.map.get(i) {
                return i.clone();
            }
        }
        panic!("Somewhere something doesn't have any events!");
    }
}

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
            $(pub $name: Option<$rawtype>,)*
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
    background:     Drawable from String = Drawable::from("#222233".to_string()),

    black:          Drawable from String = Drawable::from("#000000".to_string()),
    red:            Drawable from String = Drawable::from("#AA0000".to_string()),
    green:          Drawable from String = Drawable::from("#00AA00".to_string()),
    yellow:         Drawable from String = Drawable::from("#AAAA00".to_string()),
    blue:           Drawable from String = Drawable::from("#0000AA".to_string()),
    magenta:        Drawable from String = Drawable::from("#AA00AA".to_string()),
    cyan:           Drawable from String = Drawable::from("#00AAAA".to_string()),
    white:          Drawable from String = Drawable::from("#AAAAAA".to_string()),

    bright_black:   Drawable from String = Drawable::from("#777777".to_string()),
    bright_red:     Drawable from String = Drawable::from("#FF0000".to_string()),
    bright_green:   Drawable from String = Drawable::from("#00FF00".to_string()),
    bright_yellow:  Drawable from String = Drawable::from("#FFFF00".to_string()),
    bright_blue:    Drawable from String = Drawable::from("#0000FF".to_string()),
    bright_magenta: Drawable from String = Drawable::from("#FF00FF".to_string()),
    bright_cyan:    Drawable from String = Drawable::from("#00FFFF".to_string()),
    bright_white:   Drawable from String = Drawable::from("#FFFFFF".to_string()),

    warn:           f64 from f64 = f64::MAX,
    critical:       f64 from f64 = f64::MAX,
    dim:            f64 from f64 = f64::MIN,

    font:           String from String = "default".to_string(),

    command:        Command from Value = Command::from(Value::String("".to_string())),
    action:         Command from Value = Command::from(Value::String("".to_string())),
    border_factor:  f32 from f32 = 0.75,
    interval:       f32 from f32 = 5.0
);

// Bar properties
prop_struct!(BarProps, BarPropsCurrent, BarConfigProps, 

    alignment:      Direction from String = Direction::from("N".to_string()),
    height:         u16 from u16 = 24,
    screenwidth:    f32 from f32 = 1.0,
    xoff:           i16 from i16 = 0,
    yoff:           i16 from i16 = 0,
    solid:          bool from bool = true,
    above:          bool from bool = false,
    below:          bool from bool = false,
    visible:        bool from bool = true
);

