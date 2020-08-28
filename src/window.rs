
use std::error::Error;
use std::collections::HashMap;

use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt;

// Just an alias for convenience
pub trait XConnection: Connection + ConnectionExt {}
impl<T: Connection + ConnectionExt> XConnection for T {}

pub struct Window<'a, T: XConnection> {
    // Maybe change this in the future
    pub window: u32,
    pub colormap: u32,
    pub conn: &'a T,
    pub fontconfig: fontconfig::Fontconfig,

    screen: &'a Screen
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self{r,g,b,a}
    }

    pub fn from(s: String) -> Self {
        if s.len() != 7 || &s[0..1] != "#" {
            panic!("Only #XXXXXX format is currently acceptable")
        }
        let r = u8::from_str_radix(&s[1..3], 16).unwrap();
        let g = u8::from_str_radix(&s[3..5], 16).unwrap();
        let b = u8::from_str_radix(&s[5..7], 16).unwrap();
        Self{r,g,b, a: 255}
    }

    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}


pub enum Drawable {
    Color(Color)
}

impl Drawable {
    pub fn from(s: String) -> Self { // TODO Error handling, as usual
        Drawable::Color(Color::from(s))
    }

    fn image(&self, _x: i16, _y: i16, width: u16, height: u16, _maxheight: u16) -> Vec<u8> {
        match self {
            Self::Color(c) => {
                let size = width as usize * height as usize;
                let mut v = Vec::with_capacity(size * 4);

                for _ in 0..size {
                    v.extend(&[c.b, c.r, c.g, c.a]);
                }
                v
            }
        }
    }

    pub fn draw_bg<T: XConnection>(&self, window: &Window<T>, x: i16, y: i16, width: u16, height: u16)
        -> Result<(), Box<dyn Error>> 
    {
        match self {
            Drawable::Color(c) => {
                let gc = window.conn.generate_id()?;

                window.conn.create_gc(gc, window.window, &CreateGCAux::new().foreground(c.as_xcolor()))?;

                let rect = Rectangle {x,y,width,height};
                window.conn.poly_fill_rectangle(window.window, gc, &[rect])?;
                
                window.conn.flush()?;

                window.conn.free_gc(gc)?;
            }
        }
        Ok(())
    }

    fn draw_image<T: XConnection>(&self, window: &Window<T>, x: i16, y: i16, width: u16, height: u16, data: &Vec<u8>) -> Result<(), Box<dyn Error>> {
        
        let gc = window.conn.generate_id()?;
        window.conn.create_gc(gc, window.window, &CreateGCAux::new())?;

        window.conn.put_image(
            ImageFormat::ZPixmap, 
            window.window, 
            gc, 
            width, 
            height,
            x,
            y,
            0, 
            24, 
            &data)?;
        
        window.conn.free_gc(gc)?;
        Ok(())
    }

    pub fn draw_fg<T: XConnection>(&self, window: &Window<T>, x: i16, y: i16, height: u16, border_factor: f32, font: &crate::font::Font, background: &Drawable, text: &String)
        -> Result<u16, Box<dyn Error>> 
    {
        match self {
            Drawable::Color(_) => {

                let fgheight = font.height((height as f32 * border_factor).ceil() as _);
                let fgy = y + ((height - fgheight) / 2) as i16;
                
                let (glyphs, width) = font.glyphs_and_width(text, fgheight);
                
                let fg =     self      .image(x,fgy,width,fgheight,height);
                let mut bg = background.image(x,fgy,width,fgheight,height);

                font.draw_text(width, glyphs, &fg, &mut bg)?;

                self.draw_image(window, x, fgy, width, fgheight, &bg)?;
                background.draw_bg(window, x, y, width, (fgy - y) as _)?;
                background.draw_bg(window, x, fgy+fgheight as i16, width, (height - fgy as u16 - fgheight) as _)?;

                Ok(width)
            }
        }
    }

}

#[derive(Copy, Clone)]
pub struct Direction {
    // -1 - left, 0 - center, 1 - right
    pub xdir: i8,
    // -1 - top, 0 - center, 1 - bottom
    pub ydir: i8
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

pub struct WindowGeometry {
    pub dir: Direction,
    pub xoff: i16,
    pub yoff: i16,
    pub w: u16,
    pub h: u16
}

impl WindowGeometry {
    pub fn new() -> Self {
        Self {dir: Direction::from("N".to_owned()), xoff:0,yoff:0,w:0,h:0}
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

    pub fn cropped(&self, x: i16, y: i16, w: u16, h: u16) -> Self {
        let xoff = if self.dir.xdir == 0 {self.xoff + x} else {self.xoff.abs() + x.abs() };
        let yoff = if self.dir.ydir == 0 {self.yoff + y} else {self.yoff.abs() + y.abs() };

        Self {dir: self.dir, xoff, yoff, w, h}
    }

    pub fn has_point(&self, px: i16, py: i16, scrw: u16, scrh: u16) -> bool {
        let (x,y,w,h) = self.on_screen(scrw, scrh);
        px >= x && py >= y && px <= x + w as i16 && py <= y + h as i16
    }

    pub fn strut(&self) -> [u32; 12] {
        [
            0,
            0,
            if self.dir.ydir == -1 {(self.h as i16 + self.yoff) as u32} else {0},
            if self.dir.ydir ==  1 {(self.h as i16 + self.xoff) as u32} else {0},
            0,0,0,0,0,0,0,0
        ]
    }
}

impl<T: XConnection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen: &'a Screen, geom: WindowGeometry) -> Result<Window<'a, T>, Box<dyn Error>> {
                
        let window = conn.generate_id()?;

        let (x,y,w,h) = geom.on_screen(screen.width_in_pixels, screen.height_in_pixels);

        println!("Window geom: {} {} {} {}", x,y,w,h);

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           x,y,w,h, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new()
                                .background_pixel(Color::new(255,100,200,150).as_xcolor())
                                .event_mask(EventMask::ButtonPress
                                          | EventMask::ButtonRelease))?;


        let colormap = screen.default_colormap;
        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar")?;

        conn.flush()?;

        let fontconfig = fontconfig::Fontconfig::new().unwrap();

        let wnd = Window {window, colormap, conn, fontconfig, screen};

        wnd.configure(geom)?;

        Ok(wnd)
    }

    pub fn configure(&self, geom: WindowGeometry) -> Result<(), Box<dyn Error>> {

        let (x,y,w,h) = geom.on_screen(self.screen.width_in_pixels, self.screen.height_in_pixels);


        self.set_atom32(b"_NET_WM_WINDOW_TYPE", PropMode::Replace, AtomEnum::ATOM, 
                       &[self.get_atom(b"_NET_WM_WINDOW_TYPE_DOCK")?])?;
        self.set_atom32(b"_NET_WM_DESKTOP", PropMode::Replace, AtomEnum::CARDINAL, 
                       &[0xFFFFFFFF])?;
        self.set_atom32(b"_NET_WM_STATE", PropMode::Append, AtomEnum::ATOM, 
                       &[self.get_atom(b"_NET_WM_STATE_STICKY")?,
                         self.get_atom(b"_NET_WM_STATE_STAYS_ON_TOP")?])?;
        self.set_atom32(b"_NET_WM_ALLOWED_ACTIONS", PropMode::Replace, AtomEnum::ATOM, 
                       &[])?;



        self.set_atom32(b"_NET_WM_STRUT", PropMode::Replace, AtomEnum::CARDINAL, 
                       &geom.strut()[0..4])?;
        self.set_atom32(b"_NET_WM_STRUT_PARTIAL", PropMode::Replace, AtomEnum::CARDINAL, 
                       &geom.strut())?;

        self.conn.map_window(self.window)?;

        // Ensure window's position
        self.conn.configure_window(self.window, &ConfigureWindowAux::new().x(x as i32).y(y as i32).width(w as u32).height(h as u32))?;
        
        self.flush()?;
        Ok(())
    }

    pub fn get_atom(&self, name: &[u8]) -> Result<Atom, Box<dyn Error>> {
        Ok(self.conn.intern_atom(false, name)?.reply()?.atom)
    }

    pub fn set_atom8(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u8]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property8(mode, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn set_atom32(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u32]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property32(mode, self.window, atom, atype, data)?;
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
