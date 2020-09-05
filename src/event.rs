
use x11rb::protocol::Event as XEvent;

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone)]
pub enum Event {
    Default,
    OnHover,
    ButtonPressAny,
    ButtonPressContAny,
    ButtonReleaseAny,
    ButtonReleaseContAny,
}

impl Event {
    pub fn from(event: &String, settings: &String) -> Self { // TODO Errors
        match &event[..] {
            "default" => Self::Default,
            "on_hover" => Self::OnHover,
            "on_press" => Self::ButtonPressAny,
            "on_press_cont" => Self::ButtonPressContAny,
            "on_release" => Self::ButtonReleaseAny,
            "on_release_cont" => Self::ButtonReleaseContAny,
            _ => {panic!("Invalid event {}.{}", event, settings)}
        }
    }

    pub fn events_from(ev: XEvent) -> Vec<Self> {
        match ev {
            XEvent::Expose(_) => vec![Self::Default],
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
            Self::OnHover => 200,
            Self::Default => 1000
        }
    }

    pub fn mouse_dependent(&self) -> bool {
        match self {
            Self::OnHover => true,
            Self::ButtonPressAny => true,
            Self::ButtonReleaseAny => true,
            Self::ButtonPressContAny => true,
            Self::ButtonReleaseContAny => true,
            _ => false
        }
    }
}

