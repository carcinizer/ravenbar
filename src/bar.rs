
use crate::window::*;
use crate::config;
use crate::font::Font;

use std::collections::HashMap;
use x11rb::protocol::Event as XEvent;

#[derive(PartialEq, Eq, Debug, Hash)]
pub enum Event {
    Default,
    OnHover
}



impl From<&String> for Event {
    fn from(s: &String) -> Self { // TODO Errors
        match &s[..] {
            "default" => Self::Default,
            "on_hover" => Self::OnHover,
            _ => {panic!("Invalid event {}", s)}
        }
    }
}
impl From<Option<XEvent>> for Event {
    fn from(ev_opt: Option<XEvent>) -> Self {
        match ev_opt {
            Some(ev) => Self::Default,
            None => Self::Default
        } 
    }
}

enum Command {
    None,
    Shell(String)
}

impl Command {
    fn from(s: String) -> Self {
        Command::Shell(s.to_owned())
    }

    fn execute(&self) -> Result<String, run_script::ScriptError> {
        match self {
            Self::Shell(s) => {
                let (code, output, error) = run_script::run_script!(s)?;
                if code != 0 {
                    eprintln!("WARNING: '{}' returned {}", s, code);
                }
                if error.chars()
                         .filter(|x| !x.is_control())
                         .collect::<String>() != String::new() {
                    
                    eprintln!("WARNING: '{}' wrote to stderr:", s);
                    eprintln!("{}", error);
                }
                Ok(output)
            }
            _ => Ok(String::new())
        }
    }
}

struct Prop<T> {
    map: HashMap<Event, T>
}

impl<T> Prop<T> {
    fn get(&self, events: &Vec<Event>) -> &T {
        for i in events.iter() {
            if let Some(x) = self.map.get(i) {
                return x;
            }
        }
        panic!("Somewhere something doesn't have any events!");
    }
}


macro_rules! prop {
    ($var:expr, $member:ident, $type:ident, $default:expr) => {{
        let mut map = HashMap::new();
        map.insert(Event::Default, $default);
        for (k,v) in $var.iter() {
            if let Some(x) = &v.$member {
                map.insert(Event::from(k), $type::from(x.clone()));
            }
        }
        Prop {map}
    }}
}


struct WidgetProps {
    foreground: Prop<Drawable>,
    background: Prop<Drawable>,
    command: Prop<Command>,
    border_factor: Prop<f32>
}

struct Widget {
    props : WidgetProps,

    width_min: u16,
    width_max: u16
}

struct BarProps {
    alignment: Prop<Direction>,
    height: Prop<u16>
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: BarProps,
    window: &'a Window<'a, T>,
    font: Font<'a>
}

impl<'a, T: XConnection> Bar<'a, T> {

    pub fn create(cfg: config::BarConfig, window: &'a Window<'a, T>) -> Self {

        let props = BarProps{
            alignment: prop!(cfg.props, alignment, Direction, Direction::from("NW".to_owned())), 
            height: prop!(cfg.props, height, u16, 25)};

        let widgets = cfg.widgets.iter()
            .map( |widget| Widget {
                props: WidgetProps {
                    foreground: prop!(widget.props, foreground, Drawable, Drawable::from("#FFFFFF".to_owned())),
                    background: prop!(widget.props, background, Drawable, Drawable::from("#222233".to_owned())),
                    command: prop!(widget.props, command, Command, Command::None),
                    border_factor: prop!(widget.props, border_factor, f32, 0.9),
                },
                width_min: 0, width_max:0
            }).collect();

        let font = Font::new("noto mono", &window.fontconfig).unwrap(); // TODO - font from file

        Self {props, widgets, window, font}
    }

    pub fn refresh(&mut self, events: Vec<Event>) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut widget_cursor = 0;
        let bar = &self.props;
        let e = &events;

        for i in self.widgets.iter_mut() {

            let props = &i.props;

            let text = props.command.get(e).execute()?;

            let width = props.foreground.get(e).draw_fg(
                self.window, 
                widget_cursor,
                0, 
                *bar.height.get(e), 
                *props.border_factor.get(e), 
                &self.font, 
                &props.background.get(e), 
                &text)?;

            let avg_char_width: u16 = width as u16 / text.len() as u16;

            if width > i.width_max || width < i.width_min {
                i.width_min = width - avg_char_width * 2;
                i.width_max = width + avg_char_width * 2;
            }

            props.background.get(e).draw_bg(self.window, widget_cursor + width as i16, 0, i.width_max - width, *bar.height.get(e))?;

            widget_cursor += i.width_max as i16;
        }

        self.window.configure(WindowGeometry{xoff: 0, yoff: 0, w: widget_cursor as u16, h: *bar.height.get(e), dir: bar.alignment.get(e).clone()})?;

        Ok(())
    }
}


