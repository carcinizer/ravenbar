
use std::error::Error;
use std::fs::File;
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Value, json, from_reader, from_value, to_writer_pretty, Map};

pub fn write_default_config(file: &str) -> Result<(), Box<dyn Error>> {
    let default_json = json!({
        
        "alignment" : "NE",
        "height" : 20,

        "defaults" : {
            "background": "#222233",
            "background.on_hover": "#333344",

            "foreground": "#FFFFFF"
        },

        "widgets" : [
            {
                "command": "date +%H:%M",
                "command.on_hover": "date +%H:%M:%S" // TODO TEST?
            },
            {
                "command": "echo This is a test"
            }
        ]
    });

    let cfg = File::create(file)?;
    to_writer_pretty(cfg, &default_json)?;
    Ok(())
}


#[derive(Deserialize, Clone, Debug)]
struct BarConfigProps {
    alignment: Option<String>,
    height: Option<i32>
}


#[derive(Deserialize, Clone, Debug)]
struct BarConfigWidgetProps {
    background: Option<String>,
    foreground: Option<String>,
    command: Option<String>
}

#[derive(Debug)]
struct BarConfigWidget {
    props: HashMap<String, BarConfigWidgetProps>
}

#[derive(Debug)]
pub struct BarConfig {
    props: HashMap<String, BarConfigProps>,
    widgets: Vec<BarConfigWidget>
}


impl BarConfig {
    pub fn new(filename: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(filename)?;

        let mut default_widget = BarConfigWidget::new();
        let mut bar_props_proto = HashMap::<String, Map<String, Value>>::new();
        let mut widget_arr = Vec::<Value>::new();

        let values : Value = from_reader(file)?;

        if let Value::Object(barconfig) = values {
            for (key, val) in barconfig.iter() {
                let (prop, event) = split_key(key);
                
                match &*prop {
                    "defaults" => {
                        if event != "default" {
                            panic!("Events are unapplicable to 'defaults' section");
                        }
                        default_widget = BarConfigWidget::create(val)?;
                    }
                    "widgets" => {
                        if let Value::Array(arr) = val {
                            widget_arr = arr.clone();
                        }
                        else {panic!("'widgets' value must be an array")}
                    }
                    _ => {
                        bar_props_proto.entry(event).or_default().insert(prop, val.to_owned());
                    }
                }
            }
        }
        else {panic!("Bar config does not contain a JSON root object")} // TODO Result
        

        let mut props = bar_props_proto
                        .iter().map(| (k,v)| 
                            (k.to_owned(), 
                             from_value::<BarConfigProps>(Value::Object(v.to_owned()))
                                .unwrap()) 
                        ).collect();
        

        let widgets: Vec<BarConfigWidget> = widget_arr
                        .iter().map(|v| {
                            let mut widget = BarConfigWidget::create(v).unwrap();
                            
                            for (k, p) in widget.props.iter_mut() {
                                p.mix(default_widget.props.get(k)
                                        .unwrap_or(&BarConfigWidgetProps::new()));
                            };
                            for (k, p) in default_widget.props.iter() {
                                if let None = widget.props.get(k) {
                                    widget.props.insert(k.to_owned(), p.to_owned());
                                }
                            }
                            widget
                        }).collect();

        Ok(BarConfig {props, widgets})
    }
}

impl BarConfigWidget {
    fn new() -> Self {
        Self { props: HashMap::new() }
    }

    fn create(obj: &Value) -> Result<Self, serde_json::Error> {

        let mut widget_props_proto: HashMap<String, Map<String, Value>> = HashMap::new();

        if let Value::Object(values) = obj {
            for (key, val) in values {
                let (prop, event) = split_key(key);
                
                widget_props_proto.entry(event).or_default().insert(prop, val.to_owned());
            }

            Ok(Self { 
                props: widget_props_proto
                        .iter().map(|(k,v)| 
                            (k.clone(), from_value(Value::Object(v.clone())).unwrap()) 
                        ).collect() })
        }
        else {panic!("Widget must be an object")} //TODO Error handling
    }
}


fn split_key(key: &str) -> (String, String) { // TODO Result
    let words: Vec<&str> = key.split('.').collect();

    let prop = words[0].to_owned();
    let event = match words.len() {
        1 => "default".to_owned(),
        2 => words[1].to_owned(),
        _ => {panic!("Key {} has more than 1 dot, which is not allowed", key);}
    };
    (prop, event)
}


fn mix_options<T: Clone>(parent: &Option<T>, child: &Option<T>) -> Option<T> {
    match child {
        Some(x) => Some(x.clone()),
        None => match parent {Some(y) => Some(y.clone()), None => None}
    }
}

impl BarConfigWidgetProps {
    pub fn new() -> Self {
        Self { 
            background: None,
            foreground: None,
            command: None
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
