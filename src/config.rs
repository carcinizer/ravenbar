
use crate::props::{BarConfigWidgetProps, BarConfigProps};

use std::error::Error;
use std::fs::{OpenOptions, File};
use std::io::Write;
use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, from_reader, from_value, Map};

extern crate dirs;

pub fn config_dir<'a>() -> std::path::PathBuf {
    match dirs::config_dir() {
        Some(x) => x.join("ravenbar"),
        None => {panic!("Failed to find .config directory!")}
    }
}

pub fn write_default_config(file: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut cfg = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)?;

    cfg.write(include_bytes!("../examples/default.json"))?;
    Ok(())
}

#[derive(Debug)]
pub struct BarConfigWidget {
    pub props: HashMap<(String, String), BarConfigWidgetProps>,
    pub template: HashMap<(String, String), String>
}

#[derive(Debug)]
pub struct BarConfig {
    pub props: HashMap<(String, String), BarConfigProps>,
    pub widgets_left: Vec<BarConfigWidget>,
    pub widgets_right: Vec<BarConfigWidget>,
    pub default_bg: String,

    pub fonts: HashMap<String, Vec<String>>
}


impl BarConfig {
    pub fn new(filename: PathBuf) -> Result<Self, Box<dyn Error>> {
        let file = File::open(filename)?;

        let mut default_widget = BarConfigWidget::new();
        let mut bar_props_proto = HashMap::<(String, String), Map<String, Value>>::new();
        let mut widget_left_arr = Vec::<Value>::new();
        let mut widget_right_arr = Vec::<Value>::new();
        let mut fonts = HashMap::<String, Vec<String>>::new();
        let mut templates = HashMap::<String, BarConfigWidget>::new();
        
        // Insert the default font
        fonts.insert("default".to_string(), vec!("Monospace".to_string()));
        // Insert the 'default' template
        templates.insert("".to_string(), BarConfigWidget::new());

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
                    "template" => {
                        match templates.insert(event.clone(), BarConfigWidget::create(val)?) {
                            None => (),
                            Some(_) => panic!("Template '{}' already exists", event)
                        }
                    }
                    "widgets_left" => {
                        if let Value::Array(arr) = val {
                            widget_left_arr = arr.clone();
                        }
                        else {panic!("'widgets' value must be an array")}
                    }
                    "widgets_right" => {
                        if let Value::Array(arr) = val {
                            widget_right_arr = arr.clone();
                        }
                        else {panic!("'widgets' value must be an array")}
                    }
                    "font" => {
                        if let Value::String(s) = val {
                            fonts.insert(event.clone(), vec!(s.clone()));
                        }
                        else if let Value::Array(a) = val {
                            let mut names = Vec::with_capacity(a.len());
                            
                            for i in a.iter() {
                                if let Value::String(x) = i {
                                    names.push(x.clone());
                                }
                                else {panic!("'font' must be either a string or an array of strings")}
                            }
                            
                            fonts.insert(event.clone(), names);
                        }
                        else {panic!("'font' must be either a string or an array of strings")}
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

        let create_widgets = |widget_arr: &Vec<Value>| widget_arr
                        .iter().map(|v| {
                            let mut widget = BarConfigWidget::create(v).unwrap();

                            // Mix with current template
                            for (k, name) in widget.template.clone().iter() {
                                match templates.get(name) {
                                    Some(t) => {widget.mix(t, Some(k));},
                                    None => panic!("Template '{}' doesn't exist", name)
                                }
                            }

                            // Mix with default template
                            let default_default_template = &String::new();
                            let default_template = widget.template
                                .get(&("default".to_string(), "".to_string()))
                                .unwrap_or(default_default_template);

                            match templates.get(default_template) {
                                Some(t) => {widget.mix(t, None);},
                                None => panic!("Template '{}' doesn't exist", default_template)
                            }

                            // Mix with 'defaults' section
                            widget.mix(&default_widget, None);

                            widget
                        }).collect();
        
        let widgets_left  = create_widgets(&widget_left_arr);
        let widgets_right = create_widgets(&widget_right_arr);

        let default_bg = match default_widget.props
            .get(&("default".to_string(), String::new())) 
        {
            Some(x) => x.background.clone().unwrap_or("#222233".to_string()),
            None => "#222233".to_string()
        };

        Ok(BarConfig {props, widgets_left, widgets_right, fonts, default_bg})
    }

    pub fn get_files_to_watch(&self) -> HashMap<PathBuf, std::time::SystemTime> {
        self.props.keys()
            .chain(self.widgets_left.iter().flat_map(|x| x.props.keys()))
            .chain(self.widgets_right.iter().flat_map(|x| x.props.keys()))
            .filter(|x| &x.0[..] == "on_file_changed")
            .map(|x| (config_dir().join(&x.1), std::fs::metadata(config_dir().join(&x.1))
                            .expect("File not found").modified()
                            .expect("Could not get file modification time")
            )).collect()
    }
}

impl BarConfigWidget {
    fn new() -> Self {
        Self { props: HashMap::<(String, String), BarConfigWidgetProps>::new(), template: HashMap::new() }
    }

    fn create(obj: &Value) -> Result<Self, serde_json::Error> {

        let mut widget_props_proto: HashMap<(String, String), Map<String, Value>> = HashMap::new();
        let mut template: HashMap<(String, String), String> = HashMap::new();

        if let Value::Object(values) = obj {
            for (key, val) in values {
                let (prop, event, settings) = split_key(key);
                
                if prop == "template" {
                    match val {
                        Value::String(s) => {template.insert((event, settings), s.to_owned());},
                        _ => {panic!("Template name must be a string");}
                    }
                } else {
                    widget_props_proto.entry((event, settings)).or_default().insert(prop, val.to_owned());
                }

            }

            Ok(Self { 
                props: widget_props_proto
                        .iter().map(|(k,v) : (&(String, String), &Map<String, Value>)| 
                            (k.clone(), from_value(Value::Object(v.clone())).unwrap()) 
                        ).collect::<HashMap<(String, String), BarConfigWidgetProps>>(),
                template
            })
        }
        else {panic!("Widget must be an object")} //TODO Error handling
    }

    fn mix(&mut self, other: &Self, filter: Option<&(String, String)>) -> &mut Self{
        for (k, p) in other.props.iter()
            .filter(|(k,_)| match filter {
                Some(s) => *k == s,
                None => true
            }) 
        {
            match self.props.get_mut(k) {
                Some(pm) => {pm.mix(p);}
                None => {self.props.insert(k.to_owned(), p.to_owned());}
            }
        }
        self
    }
}


fn split_key(key: &str) -> (String, String, String) {
    let words: Vec<&str> = key.splitn(3, '.').collect();

    let prop     = words[0].to_owned();
    let event    = words.get(1).unwrap_or(&"default");
    let settings = words.get(2).unwrap_or(&"");

    (prop, event.to_string(), settings.to_string())
}

