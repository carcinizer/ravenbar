
use super::{Event, EventTrait, EventListener};
use crate::bar::Bar;
use crate::utils::LogType;

use x11rb::protocol::xproto::*;
use x11rb::protocol::Event as XEvent;
use x11rb::protocol::xproto::{ConnectionExt as _};
use x11rb::connection::Connection;

#[derive(Debug, Clone, Hash)]
enum WindowEvent {
    Expose,
    Hover,
    ButtonPress(Option<u8>),
    ButtonPressCont(Option<u8>),
    ButtonRelease(Option<u8>),
    ButtonReleaseCont(Option<u8>)
}

pub struct WindowListener {
    button_state: [u8; 256]
}


crate::impl_hashed_simple!(WindowEvent);

impl EventTrait for WindowEvent {
    fn precedence(&self) -> u32 {
        match self {
            Self::ButtonPress(b) => 101 + add_precedence(b),
            Self::ButtonRelease(b) => 101 + add_precedence(b),
            Self::ButtonPressCont(b) => 102 + add_precedence(b),
            Self::ButtonReleaseCont(b) => 102 + add_precedence(b),
            Self::Expose => 160,
            Self::Hover => 200,
        }
    }

    fn mouse_dependent(&self) -> bool {
        match self {
            Self::Expose => false,
            _ => true
        }
    }

    fn is_expose(&self) -> bool {
        match self {
            Self::Expose => true,
            _ => false
        }
    }
}

impl EventListener for WindowListener {

    fn reported_events(&self) -> &'static[&'static str] {
        const WINDOW_EVENTS: &'static[&'static str] = &[
            "on_hover",
            "on_press",
            "on_press_cont",
            "on_release",
            "on_release_cont"
        ];
        WINDOW_EVENTS
    }

    fn event(&mut self, event: &String, settings: &String) -> Event {
        Box::new(match &event[..] {
            "on_hover" => WindowEvent::Hover,
            "on_press" => WindowEvent::ButtonPress(mouse_button(settings.clone())),
            "on_press_cont" => WindowEvent::ButtonPressCont(mouse_button(settings.clone())),
            "on_release" => WindowEvent::ButtonRelease(mouse_button(settings.clone())),
            "on_release_cont" => WindowEvent::ButtonReleaseCont(mouse_button(settings.clone())),
            _ => panic!("Unknown event {}.{} (reported by FilesListener)", event, settings.clone())
        })
    }


    fn get(&mut self, bar: &Bar, v: &mut Vec<Event>) {
        const E: &str = "Failed to poll X events";
        
        let conn = &bar.get_window().conn;
        let ev_opt = conn.poll_for_event().expect(E);
        
        if let Some(e1) = ev_opt {
            v.extend(xevents_to_events(e1));

            while let Some(e2) = conn.poll_for_event().expect(E) {
                v.extend(xevents_to_events(e2));
            }
        }
    }
}

impl WindowListener {
    pub fn new() -> Self {
        Self {button_state: [0u8; 256]}
    }
}

fn xevents_to_events(ev: XEvent) -> Vec<Event> {
    match ev {
        XEvent::Expose(_) => vec![Box::new(WindowEvent::Expose)],
        XEvent::ButtonPress(x) => vec![Box::new(WindowEvent::ButtonPress(None)), Box::new(WindowEvent::ButtonPress(Some(x.detail)))],
        XEvent::ButtonRelease(x) => vec![Box::new(WindowEvent::ButtonRelease(None)), Box::new(WindowEvent::ButtonRelease(Some(x.detail)))],
        _ => { crate::log!(LogType::Warning, "Unknown X event {:?}", ev); vec![]}
    }
}


fn mouse_button(s: String) -> Option<u8> {
    match &s[..] {
        "" => None,
        "left" => Some(1), 
        "middle" => Some(2), 
        "right" => Some(3), 
        "scroll_up" => Some(4), 
        "scroll_down" => Some(5), 
        _ => Some(u8::from_str_radix(&s, 10)
                  .expect("Mouse button must be either a number or one of: (left, middle, right, scroll_up, scroll_down)"))
    }
}

fn add_precedence(b: &Option<u8>) -> u32 {
    match b {
        Some(_) => 0,
        None => 5
    }
}
