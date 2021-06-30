
use crate::props::*;
use crate::window::*;
use crate::event::Event;
use crate::command::{CommandTrait as _, CommandSharedState};
use crate::config::{BarConfig, BarConfigWidget};
use crate::draw::{Drawable, DrawableSet, DrawFGInfo};

use std::time::Instant;

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


pub struct Bar {
    widgets_left: Vec<Widget>,
    widgets_right: Vec<Widget>,
    props: BarProps,
    default_bg: Drawable,

    current: BarPropsCurrent,

    offset: i16,
    middle_left: i16,
    middle_right: i16,
    geometry: WindowGeometry,
    fake_geometry: WindowGeometry,
    window: Window,
    cmdstate: CommandSharedState
}

impl Bar {

    pub fn create(cfg: BarConfig) -> Self {

        let props = BarProps::from(&cfg.props);

        let create_widgets = |widgets: &Vec<BarConfigWidget>| widgets.iter()
            .map( |widget| {
                let props = WidgetProps::from(&widget.props);
                let current = props.as_current(&vec![Event::Default], false);
                Widget {
                    props,
                    width_min: 0, width_max:0,
                    last_time_updated: Instant::now(),
                    last_event_updated: Event::Default,
                    last_x: 0, 
                    cmd_out: String::new(),
                    drawinfo: DrawFGInfo {x:0,y:0,width:0,height:0,fgy:0,fgheight:0,xb:0.0,yb:0.0,font:String::new()},
                    current,
                    mouse_over: false,
                    needs_redraw: false
            }}).collect();
        
        let widgets_left  = create_widgets(&cfg.widgets_left);
        let widgets_right = create_widgets(&cfg.widgets_right);

        let window = Window::new().expect("Failed to create window");

        let current = props.as_current(&vec![Event::Default], false);

        let mut bar = Self {props, widgets_left, widgets_right, window, 
            geometry: WindowGeometry::new(), fake_geometry: WindowGeometry::new(),
            current,
            cmdstate: CommandSharedState::new(),
            default_bg: Drawable::from(cfg.default_bg),
            offset: 0,
            middle_left: 10000,
            middle_right: 0
        };
        bar.refresh(vec![Event::Default], true, 0, 0);
        bar
    }

    pub fn refresh_widgets(&mut self, 
        side: bool,
        events: &Vec<Event>, 
        force: bool, 
        bar_redraw: bool, 
        mx: i16, my: i16) -> i16 
    {
        let mut widget_cursor = 0;
        
        let bar = &self.current;
        let height = bar.height;
        let e = events;

        let widgets = match side {
            true => self.widgets_left.iter_mut(),
            false => self.widgets_right.iter_mut()
        };

        for i in widgets {

            // Determine if mouse is inside widget
            let m = self.fake_geometry
                .has_point_cropped(mx, my, self.window.screen_width(), self.window.screen_height(),
                                   i.last_x, 0, i.width_max, height);

            // Get widget props and determine whether they changed
            let new_current = i.props.as_current(e,m);
            i.needs_redraw = if new_current != i.current {
                i.current = new_current;
                true
            }
            else {bar_redraw || force};

            let props = &i.current;

            // Update widget text
            if force || i.last_time_updated.elapsed().as_millis() > (props.interval * 1000.0) as u128
                     || i.last_event_updated != i.props.command.get_event(e,m) 
                     || props.command.updated(&mut self.cmdstate) {
                     
                let new_cmd_out = props.command.execute(&mut self.cmdstate);
                i.last_time_updated = Instant::now();
                i.last_event_updated = i.props.command.get_event(e,m);

                if new_cmd_out != i.cmd_out {
                    i.needs_redraw = true;
                    i.cmd_out = new_cmd_out;
                }
            }

            // Perform action
            props.action.execute(&mut self.cmdstate);
            
            // New draw info
            i.drawinfo = DrawFGInfo::new(&self.window, widget_cursor, 0, height, props.border_factor, &props.font, &i.cmd_out);

            // New widget width
            let width = i.drawinfo.width;
            let avg_char_width = if i.cmd_out.len() != 0 {
                width as u16 / i.cmd_out.len() as u16
            } else {1};

            if width > i.width_max || width < i.width_min {
                i.width_min = width - avg_char_width * 2;
                i.width_max = width + avg_char_width * 2;
            }
            
            i.mouse_over = m;
            widget_cursor += i.width_max as i16;
        }

        widget_cursor 
    }

    pub fn refresh(&mut self, events: Vec<Event>, force: bool, mx: i16, my: i16) -> Result<(), Box<dyn std::error::Error>> {
        
        let e = &events;
        
        // Determine if mouse is inside bar
        let bm = self.fake_geometry.has_point(mx, my, self.window.screen_width(), self.window.screen_height());
        
        // Get bar props and determine whether they changed
        let new_current = self.props.as_current(e,bm);

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
            xoff: *self.props.xoff.get(e,false), yoff: *self.props.yoff.get(e,false), 
            w: width as u16, h: height,
            dir: *self.props.alignment.get(e,false), 
            solid: bar.solid, above: bar.above, below: bar.below, visible: bar.visible
        };
        
        let global_redraw = if next_geom != self.geometry {
            self.geometry = next_geom;
            self.window.configure(&self.geometry)?;
            true
        }
        // Redraw on exposure
        else {events.iter().find(|x| **x == Event::Expose) != None};

        // Redraw left widgets
        for i in self.widgets_left.iter_mut() {

            if global_redraw || i.needs_redraw || i.drawinfo.x != i.last_x { 
                
                let ds = DrawableSet::from(&i.current);

                ds.draw_widget(&self.window, &i.drawinfo, 0, i.width_max, &i.cmd_out);
            }
            i.last_x = i.drawinfo.x; 
            i.needs_redraw = false;
        }
        // Redraw right widgets
        for i in self.widgets_right.iter_mut() {

            if global_redraw || i.needs_redraw || i.drawinfo.x + self.offset != i.last_x { 

                let ds = DrawableSet::from(&i.current);

                ds.draw_widget(&self.window, &i.drawinfo, self.offset, i.width_max, &i.cmd_out);
            }
            i.last_x = i.drawinfo.x + self.offset; 
            i.needs_redraw = false;
        }
        // Draw background between widget chunks
        if global_redraw  {
            self.default_bg.draw(&self.window, None, width_left, 0, (self.offset - width_left) as u16, height, height);
        }
        else { 
            if new_middle_left < self.middle_left {
                let end = self.middle_left.min(new_middle_right);
                self.middle_right = self.middle_right.max(end);

                self.default_bg.draw(&self.window, None, new_middle_left, 0, (end - new_middle_left) as u16, height, height);
            }
            if new_middle_right > self.middle_right {
                let begin = self.middle_right.max(new_middle_left);
                self.default_bg.draw(&self.window, None, begin, 0, (new_middle_right - begin) as u16, height, height);
            }
        }

        self.middle_left = new_middle_left;
        self.middle_right = new_middle_right;
        self.window.flush();

        Ok(())
    }

    pub fn get_current_events(&self) -> (Vec<Event>, i16, i16) {
        self.window.get_current_events()
    }

    pub fn flush(&self) {
        self.window.flush();
    }
}

