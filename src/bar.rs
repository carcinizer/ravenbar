
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

    fn execute(&self) -> Result<String, run_script::ScriptError> {
        match self {
            Self::Shell(s) => {
                let (code, output, error) = run_script::run_script!(s)?;
                if code != 0 {
                    eprintln!("WARNING: '{}' returned {}", s, code);
                }
                if output != "" {
                    eprintln!("WARNING: '{}' wrote to stderr:", s);
                    eprintln!("{}", error);
                }
                Ok(output)
            }
            _ => Ok(String::new())
        }
    }
}

struct WidgetProps {
    foreground: Drawable,
    background: Drawable,
    command: Command
}

struct Widget {
    props : HashMap<Event, WidgetProps>,

    width_min: u16,
    width_max: u16
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
                Widget {props, width_min: 0, width_max: 0}
            }).collect();

        let font = Font::new("noto sans", props[&Event::Default].height, &window.fontconfig).unwrap(); // TODO - font from file

        Self {props, widgets, window, font}
    }

    pub fn refresh(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut widget_cursor = 0;
        for i in self.widgets.iter_mut() {

            let props = &i.props[&event];

            let text = props.command.execute()?;
            let width = props.foreground.draw_text(self.window, widget_cursor, 0, &self.font, &text)?;
            let avg_char_width: u16 = width as u16 / text.len() as u16;

            if width > i.width_max || width < i.width_min {
                i.width_min = width - avg_char_width * 2;
                i.width_max = width + avg_char_width * 2;
            }

            widget_cursor += i.width_max as i16;
        }
        Ok(())
    }
}

