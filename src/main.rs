use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::Event;

mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];

    let wnd = window::Window::new(&conn, &screen);

    loop {
        let event = conn.wait_for_event()?;
        println!("{:?}", event);
        
        match event {
            Event::KeyPress(key) => {eprintln!("{:?}", key);},
            _ => {}
        }
    }
}
