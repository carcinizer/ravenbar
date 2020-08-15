use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::Rectangle;

mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];

    let wnd = window::Window::new(&conn, &screen)?;

    loop {
        let event = conn.wait_for_event()?;
        println!("{:?}", event);
        
        window::Drawable::Rect(window::Color::new(255,255,0,255)).draw(&wnd, Rectangle{x:10, y:10, width:100, height:100})?;

        match event {
            Event::KeyPress(key) => {eprintln!("{:?}", key);},
            _ => {}
        }
    }
}
