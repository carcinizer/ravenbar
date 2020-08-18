
use crate::window::*;
use crate::config;

use std::collections::HashMap;

//use x11rb::connection::{Connection, ConnectionExt};
 
enum Event {
    Default,
    OnHover
}

enum Command {
    Shell(String)
}

struct WidgetProps {
    foreground: Drawable,
    background: Drawable,
    command: Command
}

pub struct Widget {
    props : HashMap<Event, WidgetProps>
}

struct BarProps {
    alignment: Direction,
    height: i16
}

pub struct Bar<'a, T: XConnection> {
    widgets: Vec<Widget>,
    props: HashMap<Event, BarProps>,
    window: &'a Window<'a, T>
}

impl<T: XConnection> Bar<'_, T> {
    //pub fn create(cfg: &config::BarConfig, window: &Window<T>) -> Self {
    //
    //}
}
