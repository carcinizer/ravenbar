
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
mod draw;
mod utils;

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
        let mut files_last_changed = config.get_files_to_watch();

        let mut b = bar::Bar::create(config, &wnd)?;

        loop {
            let (x,y) = wnd.get_pointer()?;
            let ev_opt = conn.poll_for_event()?;
            
            let mut evec : Vec<Event> = vec![];
            
            if let Some(e1) = ev_opt {
                evec.extend(Event::events_from(e1));

                while let Some(e) = conn.poll_for_event()? {
                    evec.extend(Event::events_from(e));
                }
            }

            for (file, time) in files_last_changed.iter_mut() {
                let newtime = std::fs::metadata(file).expect("File not found")
                    .modified().expect("Could not get file modification time");

                if newtime > *time {
                    *time = newtime;
                    evec.push(Event::FileChanged(file.clone()));
                }
            }
            
            // Will be filtered out in bar's refresh function
            evec.push(Event::Hover);
            evec.push(Event::Default);
            evec.sort_by_key(|x: &Event| x.precedence());

            b.refresh(evec, false, x,y)?;
            
            std::thread::sleep(std::time::Duration::from_millis(16));

            conn.flush()?;
        }
    }
}
