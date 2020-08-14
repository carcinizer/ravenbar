use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;


fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let wndid = conn.generate_id()?;
    conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, wndid, screen.root,
                       0,0,300,300, 0, WindowClass::InputOutput, 0,
                       &CreateWindowAux::new().background_pixel(screen.white_pixel))?;
    conn.map_window(wndid)?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;
        println!("{:?}", event);
        
        match event {
            Event::KeyPress(key) => {eprintln!("{:?}", key);},
            _ => {}
        }
    }
}
