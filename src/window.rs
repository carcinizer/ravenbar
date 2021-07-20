
use crate::event::Event;

use std::error::Error;

use x11rb::protocol::xproto::*;
use x11rb::protocol::xproto::{ConnectionExt as _};
use x11rb::protocol::render::{ConnectionExt as _};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::wrapper::ConnectionExt;
use x11rb::atom_manager;
use x11rb::xcb_ffi::XCBConnection;

use cairo::{XCBSurface, Context};
use freetype::Library;
use fontconfig::Fontconfig;


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


pub struct Window {
    // Maybe change this in the future
    pub window: u32,
    pub colormap: u32,
    pub conn: XCBConnection,
    pub surface: XCBSurface,
    pub ctx: Context,
    pub depth: u8,
    pub fc: Fontconfig,
    pub ft: Library,

    screen: Screen,
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


#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct xcb_visualtype_t {
    pub visual_id: u32,
    pub class: u8,
    pub bits_per_rgb_value: u8,
    pub colormap_entries: u16,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub pad0: [u8; 4],
}

impl From<Visualtype> for xcb_visualtype_t {
    fn from(value: Visualtype) -> xcb_visualtype_t {
        xcb_visualtype_t {
            visual_id: value.visual_id,
            class: value.class.into(),
            bits_per_rgb_value: value.bits_per_rgb_value,
            colormap_entries: value.colormap_entries,
            red_mask: value.red_mask,
            green_mask: value.green_mask,
            blue_mask: value.blue_mask,
            pad0: [0; 4],
        }
    }
}

/// Find a `xcb_visualtype_t` based on its ID number
fn find_xcb_visualtype(conn: &impl Connection, visual_id: u32) -> Option<xcb_visualtype_t> {
    for root in &conn.setup().roots {
        for depth in &root.allowed_depths {
            for visual in &depth.visuals {
                if visual.visual_id == visual_id {
                    return Some((*visual).into());
                }
            }
        }
    }
    None
}


fn choose_visual(conn: &XCBConnection, screen_num: usize) -> (u8, Visualid) {
    let depth = 32;
    let screen = &conn.setup().roots[screen_num];

    // Try to use XRender to find a visual with alpha support
    let has_render = conn
        .extension_information(x11rb::protocol::render::X11_EXTENSION_NAME).unwrap()
        .is_some();
    if has_render {
        let formats = conn.render_query_pict_formats().unwrap().reply().unwrap();
        // Find the ARGB32 format that must be supported.
        let format = formats
            .formats
            .iter()
            .filter(|info| (info.type_, info.depth) == (x11rb::protocol::render::PictType::Direct, depth))
            .filter(|info| {
                let d = info.direct;
                (d.red_mask, d.green_mask, d.blue_mask, d.alpha_mask) == (0xff, 0xff, 0xff, 0xff)
            })
            .find(|info| {
                let d = info.direct;
                (d.red_shift, d.green_shift, d.blue_shift, d.alpha_shift) == (16, 8, 0, 24)
            });
        if let Some(format) = format {
            // Now we need to find the visual that corresponds to this format
            if let Some(visual) = formats.screens[screen_num]
                .depths
                .iter()
                .flat_map(|d| &d.visuals)
                .find(|v| v.format == format.id)
            {
                return (format.depth, visual.visual);
            }
        }
    }
    (screen.root_depth, screen.root_visual)
}


impl Window {
    pub fn new() -> Result<Window, Box<dyn Error>> {

        let (conn, screen_num) = XCBConnection::connect(None).unwrap();
        
        let screen = conn.setup().roots[screen_num].clone();

        let (depth, visual) = choose_visual(&conn, screen_num);

        let window = conn.generate_id().unwrap();
        let colormap = conn.generate_id().unwrap();

        conn.create_colormap(ColormapAlloc::None, colormap, screen.root, visual).unwrap().check()?;

        conn.create_window(depth, window, screen.root,
                           0,0,100,100, 0, WindowClass::InputOutput, visual,
                           &CreateWindowAux::new()
                                .background_pixel(x11rb::NONE)
                                .border_pixel(screen.black_pixel)
                                .colormap(colormap)
                                .event_mask(EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::Exposure)
        ).unwrap().check()?;

        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar").unwrap();

        conn.flush().unwrap();

        let atoms = Atoms::new(&conn).unwrap().reply().unwrap();

        let surface = XCBSurface::create(
            unsafe {&cairo::XCBConnection::from_raw_none(conn.get_raw_xcb_connection() as _)}, 
            &cairo::XCBDrawable(window), 
            unsafe {&cairo::XCBVisualType::from_raw_none(&mut find_xcb_visualtype(&conn, visual).unwrap() as *mut _ as _)},
            100, 100
        ).unwrap();

        let ctx = Context::new(&surface).expect("Failed to initialize Cairo");
        let fc = Fontconfig::new().expect("Failed to initialize Fontconfig");
        let ft = Library::init().expect("Failed to initialize Freetype");

        let wnd = Window {window, colormap, conn, surface, ctx, screen, depth, atoms, fc, ft};

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
        
        self.flush();
        self.surface.set_size(w.into(), h.into())?;

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

    pub fn get_current_events(&self) -> (Vec<Event>, i16, i16) {
        const E: &str = "Failed to poll X events";
        let pointer = self.conn.query_pointer(self.window).expect(E).reply().expect(E);
        let ev_opt = self.conn.poll_for_event().expect(E);
        
        let mut evec : Vec<Event> = vec![];
        
        if let Some(e1) = ev_opt {
            evec.extend(Event::events_from(e1));

            while let Some(e2) = self.conn.poll_for_event().expect(E) {
                evec.extend(Event::events_from(e2));
            }
        }
        
        (evec, pointer.root_x, pointer.root_y)
    }

    pub fn flush(&self) {
        self.conn.flush().expect("Failed to flush the connection")
    }
}
