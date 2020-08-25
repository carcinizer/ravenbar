
use crate::window::*;
use crate::config;
use crate::font::Font;

use std::collections::HashMap;
use x11rb::protocol::xproto::Rectangle;

#[derive(PartialEq, Eq, Debug, Hash)]
pub enum Event {
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

struct Widget {
    props : HashMap<Event, WidgetProps>
}

struct BarProps {
    alignment: Direction,
    height: u16
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: HashMap<Event, BarProps>,
    window: &'a Window<'a, T>,
    font: Font<'a>
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
            )).collect::<HashMap<Event, BarProps>>();

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

        let font = Font::new("noto sans", props[&Event::Default].height, &window.fontconfig).unwrap(); // TODO - font from file

        Self {props, widgets, window, font}
    }

    pub fn refresh(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut widget_cursor = 0;
        for i in self.widgets.iter() {
            let props = &i.props[&event];

            let traverse = props.foreground.draw_text(self.window, widget_cursor, 0, &self.font , "yxde".to_string())?;
            widget_cursor += traverse as i16;
        }
        Ok(())
    }
}


