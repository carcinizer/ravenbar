
use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

pub struct Window {
    window : u32,
    gc : u32
}

impl Window {
    pub fn new<T: Connection>(conn: &T, screen: &Screen) -> Result<Window, Box<dyn Error>> {
                
        let window  = conn.generate_id()?;
        let gc      = conn.generate_id()?;

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           0,0,300,300, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new().background_pixel(screen.white_pixel))?;

        conn.create_gc(gc, window, &CreateGCAux::new().foreground(screen.black_pixel))?;

        conn.map_window(window)?;
        conn.flush()?;

        Ok( Window {window, gc} )
    }
}
