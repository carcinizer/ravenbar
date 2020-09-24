
use crate::window::{Window, XConnection};
use crate::font::Font;
use crate::utils::mix_comp;

use std::error::Error;

use x11rb::protocol::xproto::*;

#[derive(Copy, Clone, PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

#[derive(Clone, PartialEq)]
pub enum Drawable {
    Color(Color),
    VGradient(Vec<Color>)
}

pub struct DrawFGInfo {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub fgy: i16,
    pub fgheight: u16,
}

impl DrawFGInfo {
    
    pub fn new(x: i16, y: i16, height: u16, border_factor: f32, font: &Font, text: &String) -> DrawFGInfo {
       
        let fgheight = (height as f32 * border_factor).ceil() as _;
        let fgy = y + ((height - fgheight) / 2) as i16;
        
        let (_, width) = font.glyphs_and_width(text, fgheight);
        
        DrawFGInfo {x,y,width,height, fgy,fgheight}
    }
}

impl Color {

    pub fn from(s: &str) -> Self {
        if (s.len() != 7 && s.len() != 9) || &s[0..1] != "#" {
            panic!("Only either #RRGGBB or #RRGGBBAA format is currently acceptable")
        }
        let r = u16::from_str_radix(&s[1..3], 16).unwrap();
        let g = u16::from_str_radix(&s[3..5], 16).unwrap();
        let b = u16::from_str_radix(&s[5..7], 16).unwrap();

        let a = if s.len() == 9 {
            u16::from_str_radix(&s[7..9], 16).unwrap()
        }
        else {255};

        // Premultiply results
        let r = (r*a/256) as u8;
        let g = (g*a/256) as u8;
        let b = (b*a/256) as u8;
        
        Self{r,g,b,a: a as u8}
    }

    pub fn sgr_color16(n: u32, b: u8) -> (u8,u8,u8) {
        match n {
            0 => (0, 0, 0), // Black
            1 => (b, 0, 0), // Red
            2 => (0, b, 0), // Green
            3 => (b, b, 0), // Yellow
            4 => (0, 0, b), // Blue
            5 => (b, 0, b), // Magenta
            6 => (0, b, b), // Cyan
            _ => (b, b, b), // White/Gray
        }
    }

    pub fn from_sgr(n: u32, params: &Vec<u32>) -> Self {
        let (r,g,b) : (u8, u8, u8) = match n {

            8 => match params.get(0) {
                // True color
                Some(2) => match params.get(1..4) {
                    Some(x) => (x[0] as _, x[1] as _, x[2] as _),
                    None => Self::sgr_color16(7,205)
                }
                // 256 color palette
                Some(5) => match params.get(1) {
                    Some(x) => {
                        if x < &8 {
                            Self::sgr_color16(*x,205)
                        }
                        else if x < &16 {
                            Self::sgr_color16(x%16, 255)
                        }
                        else if x < &232 {
                            let r = (x-16) / 36;
                            let g = ((x-16) / 6) % 6;
                            let b = (x-16) % 6;
                            
                            ((r*256/6) as _, (g*256/6) as _, (b*256/6) as _)
                        }
                        else {
                            let b = ((x - 232) * 256 / 24) as u8;
                            (b,b,b)
                        }
                    }
                    None => Self::sgr_color16(7,205)
                },
                _ => Self::sgr_color16(7, 205)
            }
            // 16 color palette
            _ => Self::sgr_color16(n, 205)
        };
        Self {r,g,b,a: 255}
    }

    pub fn white() -> Self {
        Self {r: 255, g: 255, b: 255, a: 255}
    }

    pub fn bright(&self) -> Self {
        Self {r: self.r + 50, g: self.g + 50, b: self.b + 50, a: self.a}
    }

    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn get(&self, i: usize) -> u8 {
        match i {
            0 => self.b,
            1 => self.g,
            2 => self.r,
            3 => self.a,
            _ => panic!("Tried to access {}th color field", i)
        }
    }

    pub fn mix(&self, other: &Self, factor: f32) -> Self {
        let r = mix_comp(self.r, other.r, factor);
        let g = mix_comp(self.g, other.g, factor);
        let b = mix_comp(self.b, other.b, factor);
        let a = mix_comp(self.a, other.a, factor);
        Self {r,g,b,a}
    }
}


impl Drawable {
    pub fn from(s: String) -> Self { // TODO Error handling, as usual
        let colors = s.split(";").map(|x| Color::from(x)).collect::<Vec<_>>();
        match colors.len() {
            1 => Self::Color(colors[0]),
            _ => Self::VGradient(colors)
        }
    }

    fn image(&self, _x: i16, y: i16, width: u16, height: u16, maxheight: u16) -> Vec<u8> {
        let size = width as usize * height as usize;
        let mut v = Vec::with_capacity(size * 4);
        
        match self {
            Self::Color(c) => {
                for _ in 0..size {
                    v.extend(&[c.b, c.g, c.r, c.a]);
                }
            }
            Self::VGradient(cv) => {
                for iy in 0..height {
                    for _ in 0..width {
                        let index = ((y as f32 + iy as f32)/maxheight as f32) * (cv.len() - 1) as f32;

                        let color1: Color = cv[index.floor() as usize];
                        let color2: Color = cv[index.ceil() as usize];
                        let c = color1.mix(&color2, index.fract());

                        v.extend(&[c.b, c.g, c.r, c.a]);
                    }
                }
            }
        }

        v
    }

    pub fn draw_bg<T: XConnection>(&self, window: &Window<T>, x: i16, y: i16, width: u16, height: u16, maxheight: u16)
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
            _ => self.draw_image(window, x, y, width, height, 
                             &self.image(x, y, width, height, maxheight))?
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
            window.depth, 
            &data)?;
        
        window.conn.free_gc(gc)?;
        Ok(())
    }
    

    pub fn draw_all<T: XConnection>(&self, 
        window: &Window<T>, 
        info: &DrawFGInfo, 
        offset: i16,
        width_max: u16, 
        font: &Font, 
        background: &Drawable, 
        text: &String) -> Result<(),Box<dyn Error>> 
    {
        let i = info;

        let fg     = self      .image(i.x,i.fgy,i.width,i.fgheight,i.height);
        let mut bg = background.image(i.x,i.fgy,i.width,i.fgheight,i.height);
        
        font.draw_text(i.width, i.fgheight, &text, &fg, &mut bg)?;

        let fgx = i.x + (width_max - i.width) as i16 / 2;

        // Text
        self.draw_image(window, offset + fgx, i.fgy, i.width, i.fgheight, &bg)?;

        // Top and bottom borders
        background.draw_bg(window, offset + i.x, i.y, width_max, (i.fgy - i.y) as _, i.height)?;
        background.draw_bg(window, offset + i.x, i.fgy+i.fgheight as i16, width_max, (i.height - i.fgy as u16 - i.fgheight) as _, i.height)?;
        
        // Left and right borders
        background.draw_bg(window, offset + i.x, i.fgy, (fgx - i.x) as _, i.fgheight, i.height)?;
        background.draw_bg(window, offset + fgx + i.width as i16, i.fgy, (fgx - i.x) as _, i.fgheight, i.height)?;

        Ok(())
    }

}

