
use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;


pub struct Window<'a, T: Connection> {
    window : u32,
    gc : u32,
    conn : &'a T
}

pub struct Color(u8,u8,u8,u8);

pub enum Drawable<'a> {
    Rect(Color),
    Text(Color, &'a str)
}

impl Drawable<'_> {
    pub fn draw<T: Connection>(&self, window: Window<T>, rect: Rectangle) {
        match self {
            Drawable::Rect(c) => {
                window.conn.poly_fill_rectangle(window.window, window.gc, &[rect]);
            }

            Drawable::Text(c, text) => {
                // TODO
            }
        }
    }
}

impl<T: Connection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen: &Screen) -> Result<Window<'a, T>, Box<dyn Error>> {
                
        let window  = conn.generate_id()?;
        let gc      = conn.generate_id()?;

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           0,0,300,300, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new().background_pixel(screen.white_pixel))?;

        conn.create_gc(gc, window, &CreateGCAux::new().foreground(screen.black_pixel))?;

        conn.map_window(window)?;
        conn.flush()?;

        Ok( Window {window, gc, conn} )
    }
}
