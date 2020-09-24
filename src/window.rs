
use std::error::Error;

use x11rb::protocol::xproto::*;
use x11rb::errors::ConnectionError;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt;
use x11rb::atom_manager;

// Just an alias for convenience
pub trait XConnection: Connection + ConnectionExt {}
impl<T: Connection + ConnectionExt> XConnection for T {}


atom_manager! {
    Atoms: AtomsCookie {
        _NET_WM_WINDOW_TYPE,       
        _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_DESKTOP,
        _NET_WM_STATE,
        _NET_WM_STATE_STICKY,
        _NET_WM_STATE_ABOVE,
        _NET_WM_STATE_BELOW,
        _NET_WM_ALLOWED_ACTIONS,
        _NET_WM_STRUT,
        _NET_WM_STRUT_PARTIAL,
    }
}


pub struct Window<'a, T: XConnection> {
    // Maybe change this in the future
    pub window: u32,
    pub colormap: u32,
    pub conn: &'a T,
    pub fontconfig: fontconfig::Fontconfig,
    pub depth: u8,

    screen: &'a Screen,
    atoms: Atoms
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Direction {
    // -1 - left, 0 - center, 1 - right
    pub xdir: i8,
    // -1 - top, 0 - center, 1 - bottom
    pub ydir: i8
}

#[derive(Debug, PartialEq)]
pub struct WindowGeometry {
    pub dir: Direction,
    pub xoff: i16,
    pub yoff: i16,
    pub w: u16,
    pub h: u16,
    pub solid: bool,
    pub above: bool,
    pub below: bool,
    pub visible: bool,
}

impl Direction {
    pub fn from(s: String) -> Self {
        let ydir = match &s[0..1] {
            "N" => -1,
            "S" => 1,
            _ => {panic!("{} is not a valid direction", s);}
        };
        let xdir = if s.len() == 2 {
            match &s[1..2] {
                "W" => -1,
                "E" => 1,
                _ => {panic!("{} is not a valid direction", s);}
            }
        } else {0};
        Self {xdir, ydir}
    }
}


impl WindowGeometry {
    pub fn new() -> Self {
        Self {dir: Direction::from("N".to_owned()), xoff:0,yoff:0,w:0,h:0, solid: false, above: false, below: false, visible: true}
    }

    pub fn on_screen(&self, scrw: u16, scrh: u16) -> (i16, i16, u16, u16) {

        let xoff = if self.dir.xdir == 0 {self.xoff} else {self.xoff.abs() * -self.dir.xdir as i16};
        let yoff = if self.dir.ydir == 0 {self.yoff} else {self.yoff.abs() * -self.dir.ydir as i16};

        let x = ((self.dir.xdir + 1) as i16) * (scrw - self.w) as i16 / 2 + xoff;
        let y = ((self.dir.ydir + 1) as i16) * (scrh - self.h) as i16 / 2 + yoff;
        let width = self.w;
        let height = self.h;
        
        (x,y,width,height)
    }

    pub fn has_point(&self, px: i16, py: i16, scrw: u16, scrh: u16) -> bool {
        let (x,y,w,h) = self.on_screen(scrw, scrh);
        px >= x && py >= y && px < x + w as i16 && py < y + h as i16
    }

    pub fn has_point_cropped(&self, px: i16, py: i16, scrw: u16, scrh: u16,
                                    cx: i16, cy: i16, cw: u16, ch: u16) -> bool {
        let (x,y,_,_) = self.on_screen(scrw, scrh);
        px >= x + cx && py >= y + cy && px < x + cx + cw as i16 && py < y + cy + ch as i16
    }

    fn strut(&self) -> [u32; 12] {
        if self.solid {
            [
                0,
                0,
                if self.dir.ydir == -1 {(self.h as i16 + self.yoff) as u32} else {0},
                if self.dir.ydir ==  1 {(self.h as i16 + self.yoff) as u32} else {0},
                0,0,0,0,0,0,0,0
            ]
        } else {[ 0,0,0,0,0,0,0,0,0,0,0,0 ]}
    }

    fn wm_state(&self, atoms: Atoms) -> Vec<u32> {
        let mut out = vec!(atoms._NET_WM_STATE_STICKY);

        if self.above {
            out.push(atoms._NET_WM_STATE_ABOVE);
        }
        else if self.below {
            out.push(atoms._NET_WM_STATE_BELOW);
        }
        out
    }
}

/// Get a visual with alpha, hopefully
fn get_depth_visual(screen: &Screen) -> (u8, Visualid) {
    for i in screen.allowed_depths.iter() {
        if i.depth == 32 {
           return (i.depth, i.visuals[0].visual_id);
        }
    }
    (x11rb::COPY_DEPTH_FROM_PARENT, screen.root_visual)
}

impl<T: XConnection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen_num: usize) -> Result<Window<'a, T>, Box<dyn Error>> {
        
        let screen = &conn.setup().roots[screen_num];

        let (depth, visual) = get_depth_visual(screen);

        let window = conn.generate_id()?;
        let colormap = conn.generate_id()?;

        conn.create_colormap(ColormapAlloc::None, colormap, screen.root, visual)?.check()?;

        conn.create_window(depth, window, screen.root,
                           0,0,100,100, 0, WindowClass::InputOutput, visual,
                           &CreateWindowAux::new()
                                .background_pixel(x11rb::NONE)
                                .border_pixel(screen.black_pixel)
                                .colormap(colormap)
                                .event_mask(EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::Exposure)
        )?.check()?;

        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar")?;

        conn.flush()?;

        let fontconfig = fontconfig::Fontconfig::new().unwrap();
        let atoms = Atoms::new(conn)?.reply()?;

        let wnd = Window {window, colormap, conn, fontconfig, screen, depth, atoms};

        Ok(wnd)
    }

    pub fn configure(&self, geom: &WindowGeometry) -> Result<(), Box<dyn Error>> {

        let (x,y,w,h) = geom.on_screen(self.screen.width_in_pixels, self.screen.height_in_pixels);


        self.set_atom32(self.atoms._NET_WM_WINDOW_TYPE, AtomEnum::ATOM, &[self.atoms._NET_WM_WINDOW_TYPE_DOCK])?;
        self.set_atom32(self.atoms._NET_WM_DESKTOP, AtomEnum::CARDINAL, &[0xFFFFFFFF])?;
        self.set_atom32(self.atoms._NET_WM_ALLOWED_ACTIONS, AtomEnum::ATOM, &[])?;
        
        self.set_atom32(self.atoms._NET_WM_STATE, AtomEnum::ATOM, &geom.wm_state(self.atoms))?;
        
        self.set_atom32(self.atoms._NET_WM_STRUT, AtomEnum::CARDINAL, &geom.strut()[0..4])?;
        self.set_atom32(self.atoms._NET_WM_STRUT_PARTIAL, AtomEnum::CARDINAL, &geom.strut())?;

        if geom.visible {
            self.conn.map_window(self.window)?;
        }
        else {
            self.conn.unmap_window(self.window)?;
        }

        // Ensure window's position
        let aux = &ConfigureWindowAux::new().x(x as i32).y(y as i32).width(w as u32).height(h as u32);
        self.conn.configure_window(self.window, aux)?;
        
        self.flush()?;
        Ok(())
    }

    pub fn set_atom32(&self, atom: u32, atype: AtomEnum, data: &[u32]) -> Result<(), Box<dyn Error>>{
        self.conn.change_property32(PropMode::Replace, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn screen_width(&self) -> u16 {
        self.screen.width_in_pixels
    }

    pub fn screen_height(&self) -> u16 {
        self.screen.height_in_pixels
    }

    pub fn get_pointer(&self) -> Result<(i16, i16), Box<dyn Error>> {
        let pointer = self.conn.query_pointer(self.window)?.reply()?;
        Ok((pointer.root_x, pointer.root_y))
    }

    pub fn flush(&self) -> Result<(), ConnectionError> {
        self.conn.flush()
    }
}
