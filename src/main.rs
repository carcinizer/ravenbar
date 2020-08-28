use std::error::Error;

use x11rb::connection::Connection;

mod bar;
mod font;
mod config;
mod window;

use bar::Event;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let wnd = window::Window::new(&conn, &screen, window::WindowGeometry{dir: window::Direction{xdir: 0, ydir: -1}, xoff: 0, yoff: 0, w: 500, h: 25})?;

    config::write_default_config("/home/michal/myravenbar.json")?;
    //println!("{:?}", config::BarConfig::new("/home/michal/myravenbar.json")?);
    let mut b = bar::Bar::create(config::BarConfig::new("/home/michal/myravenbar.json")?, &wnd)?;

    loop {
        let ev_opt = conn.poll_for_event()?;
        
        if let Some(e1) = ev_opt {
            let (x,y) = wnd.get_pointer()?;

            let mut evec = vec![];
            evec.extend(&Event::events_from(e1));

            while let Some(e) = conn.poll_for_event()? {
                evec.extend(&Event::events_from(e));
            }
            
            // Will be filtered out in bar's refresh function
            evec.push(Event::OnHover);
            evec.push(Event::Default);

            evec.sort_by_key(|x: &Event| x.precedence());
            
            b.refresh(evec, false, x,y)?;
            
        }
        
        std::thread::sleep(std::time::Duration::from_millis(16));

        conn.flush()?;
    }
}
