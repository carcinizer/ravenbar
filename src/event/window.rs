
use super::{Event, EventTrait, EventListener};
use crate::bar::Bar;
use crate::utils::LogType;

use x11rb::protocol::Event as XEvent;
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
    button_state: [bool; 32],
    num_buttons_pressed: usize
}


crate::impl_hashed_simple!(WindowEvent, 100020);

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

    fn event(&mut self, _cmd: &mut crate::command::CommandSharedState, event: &String, settings: &String) -> Event {
        Box::new(match &event[..] {
            "on_hover" => WindowEvent::Hover,
            "on_press" => WindowEvent::ButtonPress(mouse_button(settings.clone())),
            "on_press_cont" => WindowEvent::ButtonPressCont(mouse_button(settings.clone())),
            "on_release" => WindowEvent::ButtonRelease(mouse_button(settings.clone())),
            "on_release_cont" => WindowEvent::ButtonReleaseCont(mouse_button(settings.clone())),
            _ => panic!("Unknown event {}.{} (reported by WindowListener)", event, settings.clone())
        })
    }


    fn get(&mut self, bar: &Bar, v: &mut Vec<Event>) {
        const E: &str = "Failed to poll X events";
        
        let conn = &bar.get_window().conn;
        let ev_opt = conn.poll_for_event().expect(E);
        
        if let Some(e1) = ev_opt {
            self.xevents_to_events(e1, v);

            while let Some(e2) = conn.poll_for_event().expect(E) {
                self.xevents_to_events(e2, v);
            }
        }

        for i in 0u8..32u8 {
            v.push( Box::new( if self.button_state[i as usize] {
                WindowEvent::ButtonPressCont(Some(i))
            } else {
                WindowEvent::ButtonReleaseCont(Some(i))
            }));
        }

        v.push( Box::new( if self.num_buttons_pressed > 0 {
            WindowEvent::ButtonPressCont(None)
        } else {
            WindowEvent::ButtonReleaseCont(None)
        }));
        

        v.push(Box::new(WindowEvent::Hover))
    }
}

impl WindowListener {
    pub fn new() -> Self {
        Self {button_state: [false; 32], num_buttons_pressed: 0}
    }

    fn xevents_to_events(&mut self, ev: XEvent, v: &mut Vec<Event>) {
        let mut fallback = false;
        let warn_too_large_id = |x| {crate::log!(LogType::Warning, "Mouse button with ID above 31 pressed ({}), not registering continuous events", x);};

        match ev {
            XEvent::Expose(_) => v.push(Box::new(WindowEvent::Expose)),
            XEvent::ButtonPress(x) => {
                v.push(Box::new(WindowEvent::ButtonPress(None)));
                v.push(Box::new(WindowEvent::ButtonPress(Some(x.detail))));

                let state = self.button_state.get_mut(x.detail as usize).unwrap_or_else(|| {warn_too_large_id(x.detail); &mut fallback});
                if *state == false {
                    *state = true;
                    self.num_buttons_pressed += 1;
                }
            },
            XEvent::ButtonRelease(x) => {
                v.push(Box::new(WindowEvent::ButtonRelease(None))); 
                v.push(Box::new(WindowEvent::ButtonRelease(Some(x.detail))));

                let state = self.button_state.get_mut(x.detail as usize).unwrap_or_else(|| {warn_too_large_id(x.detail); &mut fallback});
                if *state == true {
                    *state = false;
                    self.num_buttons_pressed -= 1;
                }
            },
            _ => { crate::log!(LogType::Warning, "Unknown X event {:?}", ev); }
        }
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
