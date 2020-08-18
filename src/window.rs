
use std::error::Error;
use std::collections::HashMap;

use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt;

// Just an alias for convenience
pub trait XConnection: Connection + ConnectionExt {}
impl<T: Connection + ConnectionExt> XConnection for T {}

pub struct Window<'a, T: XConnection> {
    window : u32,
    colormap : u32,
    conn : &'a T
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color{r,g,b,a}
    }
    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

pub enum Drawable {
    Color(Color)
}

impl Drawable {
    pub fn draw_rect<T: XConnection>(&self, window: &Window<T>, rect: Rectangle)
        -> Result<(), Box<dyn Error>> 
    {
        match self {
            Drawable::Color(c) => {
                let gc = window.conn.generate_id()?;

                window.conn.create_gc(gc, window.window, &CreateGCAux::new().foreground(c.as_xcolor()))?;
                window.conn.poly_fill_rectangle(window.window, gc, &[rect])?;
                
                window.conn.flush()?;

                window.conn.free_gc(gc)?;
            }
        }
        Ok(())
    }
}


pub struct Direction {
    // -1 - left, 0 - center, 1 - right
    pub xdir: i8,
    // -1 - top, 0 - center, 1 - bottom
    pub ydir: i8
}


pub struct WindowGeometry {
    pub dir: Direction,
    pub xoff: i16,
    pub yoff: i16,
    pub w: u16,
    pub h: u16
}

impl WindowGeometry {
    pub fn on_screen(&self, scrw: u16, scrh: u16) -> (i16, i16, u16, u16) {

        let xoff = if self.dir.xdir == 0 {self.xoff} else {self.xoff.abs() * -self.dir.xdir as i16};
        let yoff = if self.dir.ydir == 0 {self.yoff} else {self.yoff.abs() * -self.dir.ydir as i16};

        let x = ((self.dir.xdir + 1) as i16) * (scrw - self.w) as i16 / 2 + xoff;
        let y = ((self.dir.ydir + 1) as i16) * (scrh - self.h) as i16 / 2 + yoff;
        let width = self.w;
        let height = self.h;
        
        (x,y,width,height)
    }

    pub fn strut(&self) -> [u32; 12] {
        [
            if self.dir.xdir == -1 {(self.w as i16 + self.xoff) as u32} else {0},
            if self.dir.xdir ==  1 {(self.w as i16 + self.xoff) as u32} else {0},
            if self.dir.ydir == -1 {(self.h as i16 + self.yoff) as u32} else {0},
            if self.dir.ydir ==  1 {(self.h as i16 + self.xoff) as u32} else {0},
            0,0,0,0,0,0,0,0
        ]
    }
}

impl<T: XConnection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen: &Screen, geom: WindowGeometry) -> Result<Window<'a, T>, Box<dyn Error>> {
                
        let window = conn.generate_id()?;

        let (x,y,w,h) = geom.on_screen(screen.width_in_pixels, screen.height_in_pixels);

        println!("Window geom: {} {} {} {}", x,y,w,h);

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           x,y,w,h, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new()
                                .background_pixel(Color::new(255,100,200,150).as_xcolor())
                                .event_mask(EventMask::Exposure
                                          | EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::PointerMotion))?;


        let colormap = screen.default_colormap;
        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar")?;

        conn.map_window(window)?;
        conn.flush()?;

        let wnd = Window {window, colormap, conn};

        wnd.set_atom32(b"_NET_WM_WINDOW_TYPE", PropMode::Replace, AtomEnum::ATOM, 
                       &[wnd.get_atom(b"_NET_WM_WINDOW_TYPE_DOCK")?])?;
        wnd.set_atom32(b"_NET_WM_DESKTOP", PropMode::Replace, AtomEnum::CARDINAL, 
                       &[0xFFFFFFFF])?;
        wnd.set_atom32(b"_NET_WM_STATE", PropMode::Replace, AtomEnum::ATOM, 
                       &[wnd.get_atom(b"_NET_WM_STATE_STICKY")?])?;



        wnd.set_atom32(b"_NET_WM_STRUT", PropMode::Replace, AtomEnum::ATOM, 
                       &geom.strut()[0..4])?;
        wnd.set_atom32(b"_NET_WM_STRUT_PARTIAL", PropMode::Replace, AtomEnum::ATOM, 
                       &geom.strut())?;

        // Ensure window's position
        wnd.conn.configure_window(wnd.window, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
        
        wnd.flush()?;

        Ok(wnd)
    }

    pub fn get_atom(&self, name: &[u8]) -> Result<Atom, Box<dyn Error>> {
        Ok(self.conn.intern_atom(false, name)?.reply()?.atom)
    }

    pub fn set_atom8(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u8]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property8(mode, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn set_atom32(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u32]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property32(mode, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn flush(&self) -> Result<(), ConnectionError> {
        self.conn.flush()
    }
}
