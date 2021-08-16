
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


fn main() {

    let opt = Opt::from_args();

    match std::fs::create_dir(config_dir()) {
        Ok(_) => Ok(()),
        Err(x) => match x.kind() {
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(x)
        }
    }.expect("Failed to check/create config directory");

    let file = std::path::PathBuf::from(config_dir()).join(opt.config + ".yml");
    
    if opt.example_config {
        config::write_default_config(file).expect("Failed to write config");
    }
    else {
        let config = config::BarConfig::new(file).expect("Failed to parse config");

        let mut b = bar::Bar::create(config);

        loop {
            b.refresh(false);
            std::thread::sleep(std::time::Duration::from_millis(16));

            b.flush();
        }
    }
}
