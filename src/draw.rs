
use crate::window::{Window, XConnection};
use crate::font::Font;
use std::error::Error;

use x11rb::protocol::xproto::*;

#[derive(Copy, Clone, PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

impl Color {

    pub fn from(s: String) -> Self {
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

    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

#[derive(Clone, PartialEq)]
pub enum Drawable {
    Color(Color)
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
            window.depth, 
            &data)?;
        
        window.conn.free_gc(gc)?;
        Ok(())
    }
    

    pub fn draw_all<T: XConnection>(&self, window: &Window<T>, info: &DrawFGInfo, width_max: u16, font: &Font, background: &Drawable, text: &String) -> Result<(),Box<dyn Error>> 
    {
        let i = info;

        match self {
            Drawable::Color(_) => {

                let fg     = self      .image(i.x,i.fgy,i.width,i.fgheight,i.height);
                let mut bg = background.image(i.x,i.fgy,i.width,i.fgheight,i.height);
                
                let (glyphs, _) = font.glyphs_and_width(text, i.fgheight);

                font.draw_text(i.width, &glyphs, &fg, &mut bg)?;

                let fgx = i.x + (width_max - i.width) as i16 / 2;

                // Text
                self.draw_image(window, fgx, i.fgy, i.width, i.fgheight, &bg)?;

                // Top and bottom borders
                background.draw_bg(window, i.x, i.y, width_max, (i.fgy - i.y) as _)?;
                background.draw_bg(window, i.x, i.fgy+i.fgheight as i16, width_max, (i.height - i.fgy as u16 - i.fgheight) as _)?;
                
                // Left and right borders
                background.draw_bg(window, i.x, i.fgy, (fgx - i.x) as _, i.fgheight)?;
                background.draw_bg(window, fgx + i.width as i16, i.fgy, (fgx - i.x) as _, i.fgheight)?;

                Ok(())
            }
        }
    }

}

