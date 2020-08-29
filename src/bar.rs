
use crate::window::*;
use crate::config;
use crate::font::Font;

use std::collections::HashMap;
use x11rb::protocol::Event as XEvent;
use std::time::{Instant, Duration};

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone)]
pub enum Event {
    Default,
    OnHover,
    ButtonPressAny,
    ButtonReleaseAny
}

impl From<&String> for Event {
    fn from(s: &String) -> Self { // TODO Errors
        match &s[..] {
            "default" => Self::Default,
            "on_hover" => Self::OnHover,
            "on_press" => Self::ButtonPressAny,
            "on_release" => Self::ButtonReleaseAny,
            _ => {panic!("Invalid event {}", s)}
        }
    }
}


impl Event {
    pub fn events_from(ev: XEvent) -> Vec<Self> {
        match ev {
            XEvent::Expose(_) => vec![Self::Default],
            XEvent::ButtonPress(_) => vec![Self::ButtonPressAny],
            XEvent::ButtonRelease(_) => vec![Self::ButtonReleaseAny],
            _ => { eprintln!("Unknown event: {:?}, reverting to default", ev); vec![Self::Default]}
        }
    }

    pub fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPressAny => 1,
            Self::ButtonReleaseAny => 1,
            Self::OnHover => 10,
            Self::Default => 1000
        }
    }

    pub fn mouse_dependent(&self) -> bool {
        match self {
            Self::OnHover => true,
            Self::ButtonPressAny => true,
            Self::ButtonReleaseAny => true,
            _ => false
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
    fn get(&self, events: &Vec<Event>, mouse_inside: bool) -> &T {
        for i in events.iter().filter(|x| mouse_inside || !x.mouse_dependent()) {
            if let Some(x) = self.map.get(i) {
                return x;
            }
        }
        panic!("Somewhere something doesn't have any events!");
    }

    fn get_event<'a>(&self, events: &Vec<Event>, mouse_inside: bool) -> Event {
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
    border_factor: Prop<f32>,
    interval: Prop<f32>
}

struct Widget {
    props : WidgetProps,


    width_min: u16,
    width_max: u16,
    last_time_updated: Instant,
    last_event_updated: Event,
    last_x: i16,
    cmd_out: String
}

struct BarProps {
    alignment: Prop<Direction>,
    height: Prop<u16>
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: BarProps,

    geometry: WindowGeometry,
    window: &'a Window<'a, T>,
    font: Font<'a>
}

impl<'a, T: XConnection> Bar<'a, T> {

    pub fn create(cfg: config::BarConfig, window: &'a Window<'a, T>) -> Result<Self, Box<dyn std::error::Error>> {

        let props = BarProps{
            alignment: prop!(cfg.props, alignment, Direction, Direction::from("NW".to_owned())), 
            height: prop!(cfg.props, height, u16, 25),
        };

        let widgets = cfg.widgets.iter()
            .map( |widget| Widget {
                props: WidgetProps {
                    foreground: prop!(widget.props, foreground, Drawable, Drawable::from("#FFFFFF".to_owned())),
                    background: prop!(widget.props, background, Drawable, Drawable::from("#222233".to_owned())),
                    command: prop!(widget.props, command, Command, Command::None),
                    border_factor: prop!(widget.props, border_factor, f32, 0.9),
                    interval: prop!(widget.props, interval, f32, 5.0),
                },
                width_min: 0, width_max:0,
                last_time_updated: Instant::now(),
                last_event_updated: Event::Default,
                last_x: 0, 
                cmd_out: String::new(),
            }).collect();

        let font = Font::new("noto mono", &window.fontconfig).unwrap(); // TODO - font from file

        let mut bar = Self {props, widgets, window, font, geometry: WindowGeometry::new()};
        bar.refresh(vec![Event::Default], true, 0, 0)?;
        Ok(bar)
    }

    pub fn refresh(&mut self, events: Vec<Event>, force: bool, mx: i16, my: i16) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut widget_cursor = 0;
        let bar = &self.props;
        let e = &events;
    
        let bm = self.geometry.has_point(mx, my, self.window.screen_width(), self.window.screen_height());
        let height = *bar.height.get(e,bm);

        for i in self.widgets.iter_mut() {

            let props = &i.props;

            // Determine if mouse is inside widget
            let m = self.geometry
                .cropped(widget_cursor, 0, i.width_max, height)
                .has_point(mx, my, self.window.screen_width(), self.window.screen_height());

            // Update widget text
            if force || i.last_time_updated.elapsed().as_millis() > (props.interval.get(e,m) * 1000.0) as u128
                     || i.last_event_updated != props.command.get_event(e,m) {
                     
                i.cmd_out = props.command.get(e,m).execute()?;
                i.last_time_updated = Instant::now();
                i.last_event_updated = props.command.get_event(e,m);
            }
            
            // Redraw
            let width = props.foreground.get(e,m).draw_fg(
                self.window, 
                widget_cursor,
                0, 
                height,
                *props.border_factor.get(e,m), 
                &self.font, 
                &props.background.get(e,m), 
                &i.cmd_out)?;

            let avg_char_width: u16 = width as u16 / i.cmd_out.len() as u16;

            if width > i.width_max || width < i.width_min {
                i.width_min = width - avg_char_width * 2;
                i.width_max = width + avg_char_width * 2;
            }

            props.background.get(e,m).draw_bg(self.window, widget_cursor + width as i16, 0, i.width_max - width, height)?;
            
            widget_cursor += i.width_max as i16;
        }
        
        let next_geom = WindowGeometry{xoff: 0, yoff: 0, w: widget_cursor as u16, h: height, dir: bar.alignment.get(e,bm).clone()};
        
        if next_geom != self.geometry {
            self.geometry = next_geom;
            self.window.configure(&self.geometry)?;
        }


        Ok(())
    }
}


