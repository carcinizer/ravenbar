
use crate::properties::*;
use crate::window::*;
use crate::event::{Event, EventListeners};
use crate::command::{CommandTrait as _, CommandSharedState};
use crate::config::{BarConfig, BarConfigWidget};
use crate::draw::{Drawable, DrawableSet, DrawFGInfo};
use crate::font::Font;
use crate::utils::Log;

use std::time::Instant;
use std::cell::RefCell;
use std::collections::HashMap;

struct Widget {
    properties: WidgetProperties,

    current: WidgetPropertiesCurrent,

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


pub struct Bar {
    widgets_left: Vec<RefCell<Widget>>,
    widgets_right: Vec<RefCell<Widget>>,
    properties: BarProperties,
    default_bg: Drawable,

    current: BarPropertiesCurrent,

    offset: i16,
    middle_left: i16,
    middle_right: i16,
    fonts: HashMap<String, Font>,
    geometry: WindowGeometry,
    fake_geometry: WindowGeometry,
    window: Window,
    cmdstate: CommandSharedState,
    event_listeners: EventListeners
}

fn create_widgets(widgets: &Vec<BarConfigWidget>) -> Vec<RefCell<Widget>> {
    widgets.iter()
        .map( |widget| {
            let properties = WidgetProperties::from(&widget.properties);
            let current = properties.as_current(&vec![Event::default()], false);
            RefCell::new(Widget {
                properties,
                width_min: 0, width_max:0,
                last_time_updated: Instant::now(),
                last_event_updated: Event::default(),
                last_x: 0, 
                cmd_out: String::new(),
                drawinfo: DrawFGInfo::default(),
                current,
                mouse_over: false,
                needs_redraw: false
        })}).collect()
}


impl Bar {

    pub fn create(cfg: BarConfig) -> Self {

        let properties = BarProperties::from(&cfg.properties);
        let mut event_listeners = EventListeners::new();

        let widgets_left  = create_widgets(&cfg.widgets_left);
        let widgets_right = create_widgets(&cfg.widgets_right);

        let window = Window::new().expect("Failed to create window");

        let current = properties.as_current(&vec![Event::default()], false);

        let fonts = cfg.fonts.iter().map(|(k,v)| {
            (k.clone(), Font::new(&window, v))
        }).collect();

        let mut bar = Self {properties, widgets_left, widgets_right, window, 
            geometry: WindowGeometry::new(), fake_geometry: WindowGeometry::new(),
            current,
            cmdstate: CommandSharedState::new(),
            default_bg: Drawable::from(cfg.default_bg),
            fonts,
            offset: 0,
            middle_left: 10000,
            middle_right: 0,
            event_listeners
        };
        bar.refresh(vec![Event::default()], true, 0, 0);
        bar
    }

    pub fn refresh_widgets(&mut self, 
        side: bool,
        events: &Vec<Event>, 
        force: bool, 
        bar_redraw: bool, 
        mx: i16, my: i16) -> i16 
    {
        let mut widget_cursor = 0i16;
        
        let bar = &self.current;
        let height = bar.height;
        let e = events;

        let widgets = match side {
            true => self.widgets_left.iter(),
            false => self.widgets_right.iter()
        };

        let mut width_change = 0;

        for i in widgets {
            let mut i = i.borrow_mut();

            // Determine if mouse is inside widget
            let m = self.fake_geometry
                .has_point_cropped(mx, my, self.window.screen_width(), self.window.screen_height(),
                                   i.last_x, 0, i.width_max, height);

            // Get widget properties and determine whether they changed
            let new_current = i.properties.as_current(e,m);
            i.needs_redraw = if new_current != i.current {
                i.current = new_current;
                true
            }
            else {bar_redraw || force || width_change != 0};

            // Update widget text
            if force || i.last_time_updated.elapsed().as_millis() > (i.current.interval * 1000.0) as u128
                     || i.last_event_updated != i.properties.command.get_event(e,m) 
                     || i.current.command.updated(&mut self.cmdstate) {
                     
                let new_cmd_out = i.current.command.execute(&mut self.cmdstate);
                i.last_time_updated = Instant::now();
                i.last_event_updated = i.properties.command.get_event(e,m);

                if new_cmd_out != i.cmd_out {
                    i.needs_redraw = true;
                    i.cmd_out = new_cmd_out;
                }
            }

            // Perform action
            i.current.action.execute(&mut self.cmdstate);
            
            if i.needs_redraw {
                // New draw info
                let ds = DrawableSet::from(&i.current);
                let font = self.get_font(&i.current.font);
                i.drawinfo = DrawFGInfo::new(&self.window, &ds, widget_cursor, 0, height, i.current.border_factor, font, &i.cmd_out);

                // New widget width
                let width = i.drawinfo.width;
                let avg_char_width = if i.cmd_out.len() != 0 {
                    width as u16 / i.cmd_out.len() as u16
                } else {1};

                if width > i.width_max || width < i.width_min {

                    let width_max_old = i.width_max;

                    i.width_min = width - avg_char_width * 2;
                    i.width_max = width + avg_char_width * 2;
                    width_change += width_max_old as i16 - i.width_max as i16;
                }
            }
            
            i.mouse_over = m;
            widget_cursor += i.width_max as i16;
        }

        widget_cursor 
    }

    fn draw_widgets(&self, widgets: &Vec<RefCell<Widget>>, global_redraw: bool, offset: i16) {

        for i in widgets.iter() {
            let mut i = i.borrow_mut();

            if global_redraw || i.needs_redraw || i.drawinfo.x + offset != i.last_x { 
                
                let ds = DrawableSet::from(&i.current);

                let font = self.get_font(&i.current.font);
                ds.draw_widget(&self.window, &i.drawinfo, font, offset, i.width_max);
            }
            i.last_x = i.drawinfo.x + offset; 
            i.needs_redraw = false;
        }
    }

    pub fn refresh(&mut self, events: Vec<Event>, force: bool, mx: i16, my: i16) {
        
        let e = &events;
        
        // Determine if mouse is inside bar
        let bm = self.fake_geometry.has_point(mx, my, self.window.screen_width(), self.window.screen_height());
        
        // Get bar properties and determine whether they changed
        let new_current = self.properties.as_current(e,bm);

        let bar_redraw = if new_current != self.current {
            self.current = new_current;
            true
        }
        else {false};

        // Refresh widgets & calculate width
        let width_left  = self.refresh_widgets(true,  &events, force, bar_redraw, mx, my);
        let width_right = self.refresh_widgets(false, &events, force, bar_redraw, mx, my);

        let bar = &self.current;
        let height = bar.height;

        let minwidth = (self.window.screen_width() as f32 * bar.screenwidth) as i16;
        let width = minwidth.max(width_left + width_right);
        self.offset = width - width_right;

        let new_middle_left = width_left;
        let new_middle_right = self.offset;

        // Recalculate geometry
        let next_geom = WindowGeometry {
            xoff: bar.xoff, yoff: bar.yoff,
            w: width as u16, h: height, 
            dir: bar.alignment.clone(), 
            solid: bar.solid, above: bar.above, below: bar.below, visible: bar.visible
        };
        // Fake geometry in order to support non-insane on-hover window events
        self.fake_geometry = WindowGeometry {
            xoff: *self.properties.xoff.get(e,false), yoff: *self.properties.yoff.get(e,false), 
            w: width as u16, h: height,
            dir: *self.properties.alignment.get(e,false), 
            solid: bar.solid, above: bar.above, below: bar.below, visible: bar.visible
        };
        
        let global_redraw = if next_geom != self.geometry {
            self.geometry = next_geom;
            self.window.configure(&self.geometry).log("bar refresh - window reconfiguration");
            true
        }
        // Redraw on exposure
        else {events.iter().find(|x| x.is_expose()) != None};

        // Redraw widgets
        self.draw_widgets(&self.widgets_left,  global_redraw, 0);
        self.draw_widgets(&self.widgets_right, global_redraw, self.offset);

        // Draw background between widget chunks
        if global_redraw  {
            self.default_bg.draw_rect(&self.window, width_left as f64, 0.0, (self.offset - width_left) as f64, height as f64, height as f64);
        }
        else { 
            if new_middle_left < self.middle_left {
                let end = self.middle_left.min(new_middle_right);
                self.middle_right = self.middle_right.max(end);

                self.default_bg.draw_rect(&self.window, new_middle_left as f64, 0.0, (end - new_middle_left) as f64, height as f64, height as f64);
            }
            if new_middle_right > self.middle_right {
                let begin = self.middle_right.max(new_middle_left);
                self.default_bg.draw_rect(&self.window, begin as f64, 0.0, (new_middle_right - begin) as f64, height as f64, height as f64);
            }
        }

        self.middle_left = new_middle_left;
        self.middle_right = new_middle_right;
        self.window.flush();
    }

    pub fn get_current_events(&self) -> (Vec<Event>, i16, i16) {
        self.window.get_current_events()
    }

    pub fn flush(&self) {
        self.window.flush();
    }

    fn get_font(&self, font: &String) -> &Font {
        if let Some(f) = self.fonts.get(font) {
            f
        }
        else if let Some(f) = self.fonts.get(&"default".to_string()) {
            f
        }
        else {panic!("Failed to get custom and fallback font")}
    }
}

