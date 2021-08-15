
mod bar;
mod font;
mod config;
mod window;
mod command;
mod properties;
mod event;
mod draw;
mod utils;

use config::config_dir;
use event::{Event, EventTrait};

use std::error::Error;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt()]
struct Opt {
    
    /// Writes example config instead of reading it (throws an error if a file already exists)
    #[structopt(long)]
    example_config: bool,

    /// Bar's config name (config will be read from ~/.config/ravenbar/<name>.yml)
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

    let file = std::path::PathBuf::from(config_dir()).join(opt.config + ".yml");
    
    if opt.example_config {
        config::write_default_config(file)?;
        Ok(())
    }
    else {
        let config = config::BarConfig::new(file)?;
        let mut files_last_changed = config.get_files_to_watch();

        let mut b = bar::Bar::create(config);

        loop {
            let (mut evec, x, y) = b.get_current_events();

            // Monitor files, TODO migrate to inotify
            for (file, time) in files_last_changed.iter_mut() {
                let newtime = std::fs::metadata(file).expect("File not found")
                    .modified().expect("Could not get file modification time");

                if newtime > *time {
                    *time = newtime;
                    evec.push(Event::FileChanged(file.clone()));
                }
            }
            
            // Will be filtered out anyway if mouse is not hovering
            evec.push(Event::Hover);
            evec.push(Event::Default);
            evec.sort_by_key(|x: &Event| x.precedence());

            b.refresh(evec, false, x,y);
            
            std::thread::sleep(std::time::Duration::from_millis(16));

            b.flush();
        }
    }
}
