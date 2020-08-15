
use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;


pub struct Window<'a, T: Connection> {
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

pub enum Drawable<'a> {
    Rect(Color),
    Text(Color, &'a str)
}

impl Drawable<'_> {
    pub fn draw<T: Connection>(&self, window: &Window<T>, rect: Rectangle) -> Result<(), Box<dyn Error>> {
        match self {
            Drawable::Rect(c) => {
                let gc = window.conn.generate_id()?;

                window.conn.create_gc(gc, window.window, &CreateGCAux::new().foreground(c.as_xcolor()))?;
                window.conn.poly_fill_rectangle(window.window, gc, &[rect])?;
                
                window.conn.flush()?;

                window.conn.free_gc(gc)?;
            }

            Drawable::Text(c, text) => {
                // TODO
            }
        }
        Ok(())
    }
}

impl<T: Connection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen: &Screen) -> Result<Window<'a, T>, Box<dyn Error>> {
                
        let window   = conn.generate_id()?;

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           0,0,300,300, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new()
                                .background_pixel(Color::new(255,100,200,150).as_xcolor())
                                .event_mask(EventMask::Exposure
                                          | EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::PointerMotion))?;


        let colormap = screen.default_colormap;

        conn.map_window(window)?;
        conn.flush()?;

        Ok( Window {window, colormap, conn} )
    }
}
