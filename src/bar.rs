
use crate::window::*;
use crate::window::Drawable;
use crate::config;
use crate::font::Font;

use std::collections::HashMap;
use x11rb::protocol::Event as XEvent;
use std::time::{Instant, Duration};

// TODO move to command.rs
use sysinfo::{System, SystemExt as _, ProcessorExt as _};

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone)]
pub enum Event {
    Default,
    OnHover,
    ButtonPressAny,
    ButtonPressContAny,
    ButtonReleaseAny,
    ButtonReleaseContAny,
}


impl Event {
    pub fn from(event: &String, settings: &String) -> Self { // TODO Errors
        match &event[..] {
            "default" => Self::Default,
            "on_hover" => Self::OnHover,
            "on_press" => Self::ButtonPressAny,
            "on_press_cont" => Self::ButtonPressContAny,
            "on_release" => Self::ButtonReleaseAny,
            "on_release_cont" => Self::ButtonReleaseContAny,
            _ => {panic!("Invalid event {}.{}", event, settings)}
        }
    }

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
            Self::ButtonPressAny => 101,
            Self::ButtonReleaseAny => 101,
            Self::ButtonPressContAny => 102,
            Self::ButtonReleaseContAny => 102,
            Self::OnHover => 200,
            Self::Default => 1000
        }
    }

    pub fn mouse_dependent(&self) -> bool {
        match self {
            Self::OnHover => true,
            Self::ButtonPressAny => true,
            Self::ButtonReleaseAny => true,
            Self::ButtonPressContAny => true,
            Self::ButtonReleaseContAny => true,
            _ => false
        }
    }
}

struct CommandGlobalInfo {
    system: sysinfo::System,
    
    last_cpu: Option<Instant>,
    last_mem: Option<Instant>,
    last_net: Option<Instant>,
}

impl CommandGlobalInfo {
    pub fn new() -> Self {
        Self {
            system: sysinfo::System::new(),

            last_cpu: None,
            last_mem: None,
            last_net: None
        }
    }
    
    fn refresh_cpu(&mut self) {
        let update = if let Some(i) = self.last_cpu {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_cpu();
            self.last_cpu = Some(Instant::now()); 
        }
    }

    fn refresh_mem(&mut self) {
        let update = if let Some(i) = self.last_mem {
            i.elapsed().as_millis() > 30
        }
        else {true};

        if update {
            self.system.refresh_memory();
            self.last_mem = Some(Instant::now()); 
        }
    }

    fn cpu(&mut self, core: &Option<usize>) -> &sysinfo::Processor {
        self.refresh_cpu();
        
        match core {
            Some(c) => {
                let a = self.system.get_processors();
                if a.len() <= *c {panic!("CPU doesn't have core {}", c)};
                &a[*c]
            }
            None => self.system.get_global_processor_info()
        }
    }

    fn cpu_usage(&mut self, core: &Option<usize>) -> String {
        format!("{:.0}%", self.cpu(core).get_cpu_usage())
    }

    fn cpu_freq(&mut self, core: &Option<usize>) -> String {
        // Getting frequency for "global processor" reports 0, use core 0 freq as a fallback
        format!("{:.2}MHz", self.cpu(&Some(core.unwrap_or(0))).get_frequency())
    }

    fn mem_usage(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_used_memory() * 1024) + "B"
    }

    fn mem_percent(&mut self) -> String {
        self.refresh_mem();
        format!("{:.2}%", self.system.get_used_memory() as f32 / self.system.get_total_memory() as f32 * 100.)
    }

    fn mem_total(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_total_memory() * 1024) + "B"
    }

    fn swap_usage(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_used_swap() * 1024) + "B"
    }

    fn swap_percent(&mut self) -> String {
        self.refresh_mem();
        format!("{:.2}%", self.system.get_used_swap() as f32 / self.system.get_total_swap() as f32 * 100.)
    }

    fn swap_total(&mut self) -> String {
        self.refresh_mem();
        human_readable(self.system.get_total_swap() * 1024) + "B"
    }
}

// TODO przeniesc do jakiegos utils.rs czy cos
pub fn human_readable(n: u64) -> String {
    let (div, suffix) : (u64, &str) = 
        if      n > (1 << 50) {(1 << 50, "Pi")}
        else if n > (1 << 40) {(1 << 40, "Ti")}
        else if n > (1 << 30) {(1 << 30, "Gi")}
        else if n > (1 << 20) {(1 << 20, "Mi")}
        else if n > (1 << 10) {(1 << 10, "Ki")}
        else {(1, "")};

    format!("{:.3}{}", n as f64 / div as f64, suffix)
}

#[derive(PartialEq, Clone)]
enum Command{
    None,
    Shell(String),
    
    CPUUsage(Option<usize>),
    CPUFreq(Option<usize>),

    MemUsage,
    MemPercent,
    MemTotal,
    
    SwapUsage,
    SwapPercent,
    SwapTotal,
}

impl Command {
    fn from(s: String) -> Self {
        if s.len() == 0 {
            Self::None
        }
        else { 
            match &s[0..1] {
            "!" => {
                    let words: Vec<&str> = s[1..].split(" ").collect();
                    
                    let arg1_num = if words.len() > 1 {
                        Some(usize::from_str_radix(words[1], 10).unwrap())
                    }
                    else {None};
                    
                    match words[0] {
                        "cpu_usage" => Self::CPUUsage(arg1_num),
                        "cpu_freq" => Self::CPUFreq(arg1_num),

                        "mem_usage" => Self::MemUsage,
                        "mem_percent" => Self::MemPercent,
                        "mem_total" => Self::MemTotal,
                        
                        "swap_usage" => Self::SwapUsage,
                        "swap_percent" => Self::SwapPercent,
                        "swap_total" => Self::SwapTotal,

                        _ => {panic!("Special command not available: {}", s)}
                    }
                }
                _ => Command::Shell(s.to_owned())
            }
        }
    }

    fn execute(&self, gi: &mut CommandGlobalInfo) -> Result<String, run_script::ScriptError> {
        match self {
            Self::Shell(s) => {

                let mut options = run_script::ScriptOptions::new();
                options.working_directory = Some(config::config_dir());

                let (code, output, error) = run_script::run_script!(s, options)?;
                if code != 0 {
                    eprintln!("WARNING: '{}' returned {}", s, code);
                }
                if !error.chars()
                    .filter(|x| !x.is_control())
                    .eq(std::iter::empty()) {
                    
                    eprintln!("WARNING: '{}' wrote to stderr:", s);
                    eprintln!("{}", error);
                }
                Ok(output)
            }
            Self::CPUUsage(core) => {
                Ok(gi.cpu_usage(core))
            }
            Self::CPUFreq(core) => {
                Ok(gi.cpu_freq(core))
            }
            Self::MemUsage => {
                Ok(gi.mem_usage())
            }
            Self::MemPercent => {
                Ok(gi.mem_percent())
            }
            Self::MemTotal => {
                Ok(gi.mem_total())
            }
            Self::SwapUsage => {
                Ok(gi.swap_usage())
            }
            Self::SwapPercent => {
                Ok(gi.swap_percent())
            }
            Self::SwapTotal => {
                Ok(gi.swap_total())
            }
            Self::None => Ok(String::new()),
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
        for ((k,s),v) in $var.iter() {
            if let Some(x) = &v.$member {
                map.insert(Event::from(k, s), $type::from(x.clone()));
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

#[derive(Clone, PartialEq)]
struct WidgetPropsCurrent {
    foreground: Drawable,
    background: Drawable,
    command: Command,
    border_factor: f32,
    interval: f32
}

struct Widget {
    props : WidgetProps,

    current: WidgetPropsCurrent,

    width_min: u16,
    width_max: u16,
    last_time_updated: Instant,
    last_event_updated: Event,
    last_x: i16,
    cmd_out: String,
    drawinfo: DrawFGInfo,
    mouse_over: bool,
    needs_redraw: bool
}

struct BarProps {
    alignment: Prop<Direction>,
    height: Prop<u16>
}

#[derive(Clone, PartialEq)]
struct BarPropsCurrent {
    alignment: Direction,
    height: u16
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: BarProps,

    current: BarPropsCurrent,

    geometry: WindowGeometry,
    fake_geometry: WindowGeometry,
    window: &'a Window<'a, T>,
    font: Font<'a>,
    cmdginfo: CommandGlobalInfo
}

impl<'a, T: XConnection> Bar<'a, T> {

    pub fn create(cfg: config::BarConfig, window: &'a Window<'a, T>) -> Result<Self, Box<dyn std::error::Error>> {

        let props = BarProps{
            alignment: prop!(cfg.props, alignment, Direction, Direction::from("NW".to_owned())), 
            height: prop!(cfg.props, height, u16, 25),
        };

        let widgets = cfg.widgets.iter()
            .map( |widget| {
                let props = WidgetProps {
                    foreground: prop!(widget.props, foreground, Drawable, Drawable::from("#FFFFFF".to_owned())),
                    background: prop!(widget.props, background, Drawable, Drawable::from("#222233".to_owned())),
                    command: prop!(widget.props, command, Command, Command::None),
                    border_factor: prop!(widget.props, border_factor, f32, 0.9),
                    interval: prop!(widget.props, interval, f32, 5.0),
                };
                let current = props.as_current(&vec![Event::Default], false);
                Widget {
                    props,
                    width_min: 0, width_max:0,
                    last_time_updated: Instant::now(),
                    last_event_updated: Event::Default,
                    last_x: 0, 
                    cmd_out: String::new(),
                    drawinfo: DrawFGInfo {x:0,y:0,width:0,height:0,fgy:0,fgheight:0},
                    current,
                    mouse_over: false,
                    needs_redraw: false
            }}).collect();

        let font = Font::new("noto mono", &window.fontconfig).unwrap(); // TODO - font from file
        let current = props.as_current(&vec![Event::Default], false);

        let mut bar = Self {props, widgets, window, font, 
            geometry: WindowGeometry::new(), fake_geometry: WindowGeometry::new(),
            current,
            cmdginfo: CommandGlobalInfo::new()
        };
        bar.refresh(vec![Event::Default], true, 0, 0)?;
        Ok(bar)
    }

    pub fn refresh(&mut self, events: Vec<Event>, force: bool, mx: i16, my: i16) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut widget_cursor = 0;
        let e = &events;
        
        // Determine if mouse is inside bar
        let bm = self.fake_geometry.has_point(mx, my, self.window.screen_width(), self.window.screen_height());
        
        // Get props and determine whether they changed
        let new_current = self.props.as_current(e,bm);
        let bar_redraw = if new_current != self.current {
            self.current = new_current;
            true
        }
        else {false};

        let bar = &self.current;
        let height = bar.height;

        for i in self.widgets.iter_mut() {

            // Determine if mouse is inside widget
            let m = self.fake_geometry
                .cropped(widget_cursor, 0, i.width_max, height)
                .has_point(mx, my, self.window.screen_width(), self.window.screen_height());

            // Get widget props and determine whetherthey changed
            let new_current = i.props.as_current(e,m);
            i.needs_redraw = if new_current != i.current {
                i.current = new_current;
                true
            }
            else {bar_redraw};

            let props = &i.current;

            // Update widget text
            if force || i.last_time_updated.elapsed().as_millis() > (props.interval * 1000.0) as u128
                     || i.last_event_updated != i.props.command.get_event(e,m) {
                     
                let new_cmd_out = props.command.execute(&mut self.cmdginfo)?;
                i.last_time_updated = Instant::now();
                i.last_event_updated = i.props.command.get_event(e,m);

                if new_cmd_out != i.cmd_out {
                    i.needs_redraw = true;
                    i.cmd_out = new_cmd_out;
                }
            }
            
            // New draw info
            i.drawinfo = DrawFGInfo::new(widget_cursor, 0, height, props.border_factor, &self.font, &i.cmd_out);

            // New widget width
            let width = i.drawinfo.width;
            let avg_char_width: u16 = width as u16 / i.cmd_out.len() as u16;
            if width > i.width_max || width < i.width_min {
                i.width_min = width - avg_char_width * 2;
                i.width_max = width + avg_char_width * 2;
            }
            
            i.mouse_over = m;
            widget_cursor += i.width_max as i16;
        }
        
        let next_geom = WindowGeometry{xoff: 0, yoff: 0, w: widget_cursor as u16, h: height, dir: bar.alignment.clone()};
        // Fake geometry in order to support non-insane on-hover window events
        self.fake_geometry = WindowGeometry{xoff: 0, yoff: 0, w: widget_cursor as u16, h: height, dir: bar.alignment.clone()};
        
        if next_geom != self.geometry {
            self.geometry = next_geom;
            self.window.configure(&self.geometry)?;
        }


        for i in self.widgets.iter_mut() {

            let props = &i.props;
            let m = i.mouse_over;
            
            // Redraw
            if i.needs_redraw || i.drawinfo.x != i.last_x { 
                let foreground = props.foreground.get(e,m);
                let width = i.drawinfo.width;

                foreground.draw_fg(self.window, &i.drawinfo, &self.font, &props.background.get(e,m), &i.cmd_out)?;

                props.background.get(e,m).draw_bg(self.window, i.drawinfo.x + width as i16, 0, i.width_max - width, height)?;
            }
            i.last_x = i.drawinfo.x; 
            i.needs_redraw = false;
        }
        
        self.window.flush()?;

        Ok(())
    }
}


impl WidgetProps {
    fn as_current(&self, e: &Vec<Event>, m: bool) -> WidgetPropsCurrent {
        WidgetPropsCurrent {
            foreground: self.foreground.get(e,m).clone(),
            background: self.background.get(e,m).clone(),
            command: self.command.get(e,m).clone(),
            border_factor: self.border_factor.get(e,m).clone(),
            interval: self.interval.get(e,m).clone()}
    }
}

impl BarProps {
    fn as_current(&self, e: &Vec<Event>, m: bool) -> BarPropsCurrent {
        BarPropsCurrent {
            alignment: self.alignment.get(e,m).clone(),
            height: self.height.get(e,m).clone()
        }
    }
}
