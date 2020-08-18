
use crate::window::*;
use crate::config;

use std::collections::HashMap;

//use x11rb::connection::{Connection, ConnectionExt};

#[derive(PartialEq, Eq, Debug, Hash)]
enum Event {
    Default,
    OnHover
}

impl Event {
    fn from(s: &String) -> Self { // TODO Errors
        match &s[..] {
            "default" => Self::Default,
            "on_hover" => Self::OnHover,
            _ => {panic!("Invalid event {}", s)}
        }
    }
}

enum Command {
    None,
    Shell(String)
}

impl Command {
    fn from(s: &String) -> Self {
        Command::Shell(s.to_owned())
    }
}

struct WidgetProps {
    foreground: Drawable,
    background: Drawable,
    command: Command
}

pub struct Widget {
    props : HashMap<Event, WidgetProps>
}

struct BarProps {
    alignment: Direction,
    height: i16
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: HashMap<Event, BarProps>,
    window: &'a Window<'a, T>
}

impl<'a, T: XConnection> Bar<'a, T> {
    pub fn create(cfg: config::BarConfig, window: &'a Window<'a, T>) -> Self {

        let props = cfg.props.iter()
            .map( |(event, prop)| (
                Event::from(event),
                BarProps {
                    alignment: Direction::from(prop.alignment.as_ref().unwrap().to_owned()),
                    height: prop.height.unwrap_or(25)
                }
            )).collect();

        let widgets = cfg.widgets.iter()
            .map( |widget| {
                let props = widget.props.iter()
                    .map( |(event, prop)| (
                        Event::from(event),
                        WidgetProps {
                            foreground: Drawable::from(prop
                                            .foreground.as_ref()
                                            .unwrap_or(&"#FFFFFF".to_owned()).to_owned()),
                            background: Drawable::from(prop
                                            .background.as_ref()
                                            .unwrap_or(&"#222233".to_owned()).to_owned()),
                            command: Command::from(&prop.command
                                            .as_ref().unwrap_or(&"".to_owned()).to_owned())
                        }
                )).collect();
                Widget {props}
            }).collect();

        Self {props, widgets, window}
    }
}
