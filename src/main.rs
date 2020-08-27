use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::Rectangle;

mod bar;
mod font;
mod config;
mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let wnd = window::Window::new(&conn, &screen, window::WindowGeometry{dir: window::Direction{xdir: 0, ydir: -1}, xoff: 0, yoff: 0, w: 500, h: 25})?;

    config::write_default_config("/home/michal/myravenbar.json")?;
    //println!("{:?}", config::BarConfig::new("/home/michal/myravenbar.json")?);
    let mut b = bar::Bar::create(config::BarConfig::new("/home/michal/myravenbar.json")?, &wnd);

    loop {
        let event = conn.wait_for_event()?;
        
        b.refresh(vec!(bar::Event::Default))?;

        match event {
            _ => {}
        }
        conn.flush()?;
    }
}
