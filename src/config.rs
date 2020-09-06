
use std::error::Error;
use std::fs::{OpenOptions, File};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde_json::{Value, json, from_reader, from_value, to_writer_pretty, Map};

use crate::props::{BarConfigWidgetProps, BarConfigProps};
extern crate dirs;

pub fn config_dir<'a>() -> std::path::PathBuf {
    match dirs::config_dir() {
        Some(x) => x.join("ravenbar"),
        None => {panic!("Failed to find .config directory!")}
    }
}

pub fn write_default_config(file: PathBuf) -> Result<(), Box<dyn Error>> {
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
                "command.on_hover": "date +%H:%M:%S", // TODO TEST?
                "interval.on_hover": 0.2
            },
            {
                "command": "echo This is a test"
            }
        ]
    });

    let cfg = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)?;

    to_writer_pretty(cfg, &default_json)?;
    Ok(())
}

#[derive(Debug)]
pub struct BarConfigWidget {
    pub props: HashMap<(String, String), BarConfigWidgetProps>
}

#[derive(Debug)]
pub struct BarConfig {
    pub props: HashMap<(String, String), BarConfigProps>,
    pub widgets: Vec<BarConfigWidget>,
    pub font: String
}


impl BarConfig {
    pub fn new(filename: PathBuf) -> Result<Self, Box<dyn Error>> {
        let file = File::open(filename)?;

        let mut default_widget = BarConfigWidget::new();
        let mut bar_props_proto = HashMap::<(String, String), Map<String, Value>>::new();
        let mut widget_arr = Vec::<Value>::new();
        let mut font = String::new();

        let values : Value = from_reader(file)?;

        if let Value::Object(barconfig) = values {
            for (key, val) in barconfig.iter() {
                let (prop, event, settings) = split_key(key);

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
                    "font" => {
                        if event != "default".to_owned() {
                            panic!("Events for fonts are currently not supported (event name: {})", event);
                        }
                        if let Value::String(s) = val {
                            font = s.clone();
                        }
                        else {panic!("'font' must be a string")}
                    }
                    _ => {
                        bar_props_proto.entry((event, settings)).or_default().insert(prop, val.to_owned());
                    }
                }
            }
        }
        else {panic!("Bar config does not contain a JSON root object")} // TODO Result
        
        // Convert bar props from raw to intermediate form 
        let props : HashMap<(String, String), BarConfigProps> = bar_props_proto
                        .iter().map(|(k,v)| 
                            (k.to_owned(), 
                             from_value::<BarConfigProps>(Value::Object(v.to_owned()))
                                .unwrap()) 
                        ).collect();

        let widgets: Vec<BarConfigWidget> = widget_arr
                        .iter().map(|v| {
                            let mut widget = BarConfigWidget::create(v).unwrap();
                            
                            // Add the rest of events that exist in 'defaults'
                            // section but not in the widget
                            for (k, p) in default_widget.props.iter() {
                                if let None = widget.props.get(k) {
                                    widget.props.insert(k.to_owned(), p.to_owned());
                                }
                            }
                            // Mix props with those from 'defaults' section for each event
                            for (k, p) in widget.props.iter_mut() {
                                p.mix(default_widget.props.entry(k.to_owned()).or_default());
                            }
                            widget
                        }).collect();

        Ok(BarConfig {props, widgets, font})
    }

}

impl BarConfigWidget {
    fn new() -> Self {
        Self { props: HashMap::new() }
    }

    fn create(obj: &Value) -> Result<Self, serde_json::Error> {

        let mut widget_props_proto: HashMap<(String, String), Map<String, Value>> = HashMap::new();

        if let Value::Object(values) = obj {
            for (key, val) in values {
                let (prop, event, settings) = split_key(key);
                
                widget_props_proto.entry((event, settings)).or_default().insert(prop, val.to_owned());
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


fn split_key(key: &str) -> (String, String, String) { // TODO Result
    let words: Vec<&str> = key.split('.').collect();

    let prop = words[0].to_owned();
    let event = match words.len() {
        1 => "default".to_owned(),
        _ => words[1].to_owned()
    };
    let settings = match words.len() {
        1 => "".to_owned(),
        2 => "".to_owned(),
        _ => words[2..].join(".")
    };
    (prop, event, settings)
}

