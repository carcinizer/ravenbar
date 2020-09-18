
use x11rb::protocol::Event as XEvent;
use crate::config::config_dir;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub enum Event {
    Default,
    Expose,
    Hover,
    ButtonPressAny,
    ButtonPressContAny,
    ButtonReleaseAny,
    ButtonReleaseContAny,
    FileChanged(std::path::PathBuf),
}

impl Event {
    pub fn from(event: &String, settings: &String) -> Self { // TODO Errors
        match &event[..] {
            "default" => Self::Default,
            "on_hover" => Self::Hover,
            "on_press" => Self::ButtonPressAny,
            "on_press_cont" => Self::ButtonPressContAny,
            "on_release" => Self::ButtonReleaseAny,
            "on_release_cont" => Self::ButtonReleaseContAny,
            "on_file_changed" => Self::FileChanged(config_dir().join(settings)),
            _ => {panic!("Invalid event {}.{}", event, settings)}
        }
    }

    pub fn events_from(ev: XEvent) -> Vec<Self> {
        match ev {
            XEvent::Expose(_) => vec![Self::Expose],
            XEvent::ButtonPress(_) => vec![Self::ButtonPressAny],
            XEvent::ButtonRelease(_) => vec![Self::ButtonReleaseAny],
            _ => { eprintln!("Unknown event: {:?}, reverting to default", ev); vec![Self::Default]}
        }
    }

    pub fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPressAny => 101,
            Self::ButtonReleaseAny => 101,
            Self::ButtonPressContAny => 102,
            Self::ButtonReleaseContAny => 102,
            Self::FileChanged(_) => 104,
            Self::Expose => 105,
            Self::Hover => 200,
            Self::Default => 1000
        }
    }

    pub fn mouse_dependent(&self) -> bool {
        match self {
            Self::Hover => true,
            Self::ButtonPressAny => true,
            Self::ButtonReleaseAny => true,
            Self::ButtonPressContAny => true,
            Self::ButtonReleaseContAny => true,
            _ => false
        }
    }
}

