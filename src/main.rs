
use std::error::Error;

use x11rb::connection::Connection;
use structopt::StructOpt;

mod bar;
mod font;
mod config;
mod window;
mod command;
mod props;
mod event;

use config::config_dir;
use event::Event;

#[derive(StructOpt)]
#[structopt()]
struct Opt {
    
    /// Writes example config instead of reading it (throws an error if a file already exists)
    #[structopt(long)]
    example_config: bool,

    /// Bar's config name (config will be read from ~/.config/ravenbar/<name>.json)
    #[structopt(name="CONFIGNAME")]
    config: String,
}


fn main() -> Result<(), Box<dyn Error>> {

    let opt = Opt::from_args();

    match std::fs::create_dir(config_dir()) {
        Ok(_) => Ok(()),
        Err(x) => match x.kind() {
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(x)
        }
    }?;

    let file = std::path::PathBuf::from(config_dir()).join(opt.config + ".json");

    
    if opt.example_config {
        config::write_default_config(file)?;
        Ok(())
    }
    else {
        let (conn, screen_num) = x11rb::connect(None)?;
        
        let wnd = window::Window::new(&conn, screen_num)?;
        
        let config = config::BarConfig::new(file)?;

        let mut b = bar::Bar::create(config, &wnd)?;

        loop {
            let (x,y) = wnd.get_pointer()?;
            let ev_opt = conn.poll_for_event()?;
            
            let mut evec = vec![];
            
            if let Some(e1) = ev_opt {
                evec.extend(&Event::events_from(e1));

                while let Some(e) = conn.poll_for_event()? {
                    evec.extend(&Event::events_from(e));
                }
            }
            
            // Will be filtered out in bar's refresh function
            evec.push(Event::OnHover);
            evec.push(Event::Default);
            evec.sort_by_key(|x: &Event| x.precedence());

            b.refresh(evec, false, x,y)?;
            
            std::thread::sleep(std::time::Duration::from_millis(16));

            conn.flush()?;
        }
    }
}
