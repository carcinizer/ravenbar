
use super::{Event, EventTrait, EventListener};
use crate::config::config_dir;
use crate::bar::Bar;

use std::time::SystemTime;
use std::path::PathBuf;


#[derive(Debug, Clone, Hash)]
struct FileChanged(PathBuf);

pub struct FilesListener {
    files_to_watch: Vec<(PathBuf, SystemTime)>
}


impl EventTrait for FileChanged {
    fn precedence(&self) -> u32 {100}
    fn mouse_dependent(&self) -> bool {false}
    fn is_expose(&self) -> bool {false}
}

crate::impl_hashed_simple!(FileChanged, 100010);


impl FilesListener {
    pub fn new() -> Self {
        Self {files_to_watch: vec!()}
    }
}

impl EventListener for FilesListener {
    
    fn reported_events(&self) -> &'static[&'static str] {
        const FILES_EVENTS: &'static[&'static str] = &[&"on_file_changed"];
        FILES_EVENTS
    }

    fn event(&mut self, _cmd: &mut crate::command::CommandSharedState, event: &String, settings: &String) -> Event {
        match &event[..] {
            "on_file_changed" => {
                let dir = config_dir().join(settings);

                self.files_to_watch.push((dir.clone(), SystemTime::now()));
                Box::new(FileChanged(dir))
            },
            _ => panic!("Unknown event {}.{} (reported by FilesListener)", event, settings)
        }
    }
    fn get(&mut self, _bar: &Bar, v: &mut Vec<Event>) {
        
        // TODO migrate to inotify
        for (file, time) in self.files_to_watch.iter_mut() {
            let newtime = std::fs::metadata(file as &PathBuf).expect("File not found")
                .modified().expect("Could not get file modification time");

            if newtime > *time {
                *time = newtime;
                v.push(Box::new(FileChanged(file.clone())))
            }
        }
    }
}

