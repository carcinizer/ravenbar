
use crate::props::*;

use crate::window::*;
use crate::event::Event;
use crate::font::Font;
use crate::command::CommandGlobalInfo;
use crate::config::BarConfig;
use crate::draw::DrawFGInfo;

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

    pub fn create(cfg: BarConfig, window: &'a Window<'a, T>) -> Result<Self, Box<dyn std::error::Error>> {

        let props = BarProps::from(&cfg.props);

        let widgets = cfg.widgets.iter()
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
                    drawinfo: DrawFGInfo {x:0,y:0,width:0,height:0,fgy:0,fgheight:0},
                    current,
                    mouse_over: false,
                    needs_redraw: false
            }}).collect();

        let font = Font::new(&cfg.font[..], &window.fontconfig)?;
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
            else {bar_redraw || force};

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
        
        let global_redraw = if next_geom != self.geometry {
            self.geometry = next_geom;
            self.window.configure(&self.geometry)?;
            true
        }
        // Redraw on exposure
        else {events.iter().find(|x| **x == Event::Expose) == None};


        // Redraw
        for i in self.widgets.iter_mut() {

            let props = &i.current;
            
            if global_redraw || i.needs_redraw || i.drawinfo.x != i.last_x { 
                props.foreground.draw_all(self.window, &i.drawinfo, i.width_max, &self.font, &props.background, &i.cmd_out)?;
            }
            i.last_x = i.drawinfo.x; 
            i.needs_redraw = false;
        }
        
        self.window.flush()?;

        Ok(())
    }
}

