
use x11rb::protocol::Event as XEvent;
use crate::config::config_dir;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub enum Event {
    Default,
    Expose,
    Hover,
    ButtonPress(Option<u8>),
    ButtonPressCont(Option<u8>),
    ButtonRelease(Option<u8>),
    ButtonReleaseCont(Option<u8>),
    FileChanged(std::path::PathBuf),
}

impl Event {
    pub fn from(event: &String, settings: &String) -> Self { // TODO Errors
        match &event[..] {
            "default" => Self::Default,
            "on_hover" => Self::Hover,
            "on_press" => Self::ButtonPress(mouse_button(settings)),
            "on_press_cont" => Self::ButtonPressCont(mouse_button(settings)),
            "on_release" => Self::ButtonRelease(mouse_button(settings)),
            "on_release_cont" => Self::ButtonReleaseCont(mouse_button(settings)),
            "on_file_changed" => Self::FileChanged(config_dir().join(settings)),
            _ => {panic!("Invalid event {}.{}", event, settings)}
        }
    }

    pub fn events_from(ev: XEvent) -> Vec<Self> {
        match ev {
            XEvent::Expose(_) => vec![Self::Expose],
            XEvent::ButtonPress(x) => vec![Self::ButtonPress(None), Self::ButtonPress(Some(x.detail))],
            XEvent::ButtonRelease(x) => vec![Self::ButtonRelease(None), Self::ButtonRelease(Some(x.detail))],
            _ => { eprintln!("Unknown event: {:?}, reverting to default", ev); vec![Self::Default]}
        }
    }

    pub fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPress(b) => 101 + add_precedence(b),
            Self::ButtonRelease(b) => 101 + add_precedence(b),
            Self::ButtonPressCont(b) => 102 + add_precedence(b),
            Self::ButtonReleaseCont(b) => 102 + add_precedence(b),
            Self::FileChanged(_) => 150,
            Self::Expose => 160,
            Self::Hover => 200,
            Self::Default => 1000
        }
    }

    pub fn mouse_dependent(&self) -> bool {
        match self {
            Self::Hover => true,
            Self::ButtonPress(_) => true,
            Self::ButtonRelease(_) => true,
            Self::ButtonPressCont(_) => true,
            Self::ButtonReleaseCont(_) => true,
            _ => false
        }
    }
}

fn mouse_button(s: &String) -> Option<u8> {
    match &s[..] {
        "" => None,
        "left" => Some(1), 
        "middle" => Some(2), 
        "right" => Some(3), 
        "scroll_up" => Some(4), 
        "scroll_down" => Some(5), 
        _ => Some(u8::from_str_radix(s, 10)
                  .expect("Mouse button must be either a number or one of: (left, middle, right, scroll_up, scroll_down)"))
    }
}

fn add_precedence(b: &Option<u8>) -> u32 {
    match b {
        Some(_) => 0,
        None => 5
    }
}
