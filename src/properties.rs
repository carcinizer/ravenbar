
use crate::event::Event;
use crate::command::*;
use crate::window::Direction;
use crate::draw::Drawable;

use std::collections::HashMap;

use serde_json::Value;
use serde::Deserialize;


pub struct Property<T> {
    pub map: HashMap<Event, T>
}

impl<T> Property<T> {
    pub fn get(&self, events: &Vec<Event>, mouse_inside: bool) -> &T {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(x) = self.map.get(i) {
                return x;
            }
        }
        panic!("Bar/Widget property error (this error should be impossible)");
    }

    pub fn get_event(&self, events: &Vec<Event>, mouse_inside: bool) -> Event {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(_) = self.map.get(i) {
                return i.clone();
            }
        }
        panic!("Bar/Widget property error (this error should be impossible)");
    }
}

macro_rules! property {
    ($var:expr, $member:ident, $type:ident, $default:expr) => {{
        
        use std::collections::HashMap;
        
        let mut map = HashMap::new();
        map.insert(Event::Default, $default);
        
        for ((k,s),v) in $var.iter() {
            if let Some(x) = &v.$member {
                map.insert(Event::from(k, s), $type::from(x.clone()));
            }
        }
        Property {map}
    }}
}

/// A macro for convenient bar/widget properties declaration
/// Definitions are at the bottom of this file
macro_rules! property_struct {
    ($Properties:ident, $PropertiesCurrent:ident, $ConfigProperties:ident, 
     $( $name:ident : $type:ident from $rawtype:ident = $default:expr),*) => {
        pub struct $Properties {
            $(pub $name: Property<$type>,)*
        }

        #[derive(Clone, PartialEq)]
        pub struct $PropertiesCurrent {
            $(pub $name: $type,)*
        }

        #[derive(Deserialize, Clone, Debug)]
        pub struct $ConfigProperties {
            $(pub $name: Option<$rawtype>,)*
        }

        impl $Properties {
            pub fn as_current(&self, e: &Vec<Event>, m: bool) -> $PropertiesCurrent {
                $PropertiesCurrent {
                    $($name: self.$name.get(e,m).clone(),)*
                }
            }
        }

        impl Default for $ConfigProperties {
            fn default() -> Self {
                Self {
                    $($name: None,)*
                }
            }
        }

        impl $ConfigProperties {

            #[allow(dead_code)]
            pub fn mix(&mut self, parent: &Self) -> &Self {
                $(self.$name = parent.$name.as_ref().or(self.$name.as_ref()).cloned();)*
                self
            }
        }

        impl $Properties {
            pub fn from(config: &HashMap<(String, String), $ConfigProperties>) -> Self {
                Self {
                    $($name: property!(config, $name, $type, $default),)*
                }
            }
        }
    }
}

// Widget properties
property_struct!(WidgetProperties, WidgetPropertiesCurrent, BarConfigWidgetProperties, 
             
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
property_struct!(BarProperties, BarPropertiesCurrent, BarConfigProperties, 

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

