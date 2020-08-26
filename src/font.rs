
use fontconfig;
use rusttype;
use rusttype::{point, PositionedGlyph, Scale};
use x11rb::protocol::xproto::{ImageFormat, CreateGCAux};

use crate::window::{XConnection, Window};
use std::error::Error;

use unicode_normalization::UnicodeNormalization;

pub type FontConfig = fontconfig::Fontconfig;

pub struct Font<'a> {
    font: rusttype::Font<'a>,
}

impl Font<'_> {
    pub fn new(name: &str, fc: &FontConfig) -> Result<Self, Box<dyn Error>> {
        let fontpath = fc.find(name, None).unwrap().path;

        let font = rusttype::Font::try_from_vec(std::fs::read(fontpath)?).unwrap();

        Ok( Self {font} )
    }

    pub fn height(&self, height: u16) -> u16 {
        let vmetrics = self.font.v_metrics(scale(height));
        (vmetrics.ascent - vmetrics.descent) as _
    }

    fn ascent(&self, height: u16) -> f32 {
        self.font.v_metrics(scale(height)).ascent
    }

    fn glyphs(&self, text: &String, height: u16) -> Vec<PositionedGlyph> {
        self.font.layout(&text[..], scale(height), point(0.0, self.ascent(height)) ).collect::<Vec<_>>()
    }

    fn calc_width(&self, glyphs: &Vec<PositionedGlyph>) -> u16 {
        glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0).ceil() as _
    }

    pub fn draw_text<T: XConnection>(&self, text: &str, window: &Window<T>, x: i16, y: i16, height: u16) -> Result<u16, Box<dyn Error>> {
        
        // Get glyphs and text extents
        
        let text_nfc = text.nfc().filter(|x| !x.is_control()).collect();
        let glyphs = self.glyphs(&text_nfc, height);

        let width = self.calc_width(&glyphs);

        // Draw glyphs to buffer
        let mut data = vec![0u8; (width * height * 4) as _];
        
        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw( |x,y,v| {

                    let x = x as i16 + bb.min.x as i16;
                    let y = y as i16 + bb.min.y as i16;

                    let arrpos = (y*width as i16 + x) as usize * 4;
                    if arrpos < data.len() {

                        for i in 0..3 {
                            data[arrpos+i] = (v*255.) as _;
                        }
                    }
                })
            }
        }

        // Draw image to window
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
        Ok(width)
    }
    
}

fn scale(height: u16) -> Scale {
    Scale{x: height as f32, y: height as f32}
}
