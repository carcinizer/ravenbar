
use fontconfig;
use rusttype;
use rusttype::{point, PositionedGlyph, Scale};

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

    fn ascent(&self, height: u16) -> f32 {
        self.font.v_metrics(scale(height)).ascent
    }

    pub fn glyphs(&self, text: &String, height: u16) -> Vec<PositionedGlyph> {
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

    pub fn glyphs_and_width(&self, text: &String, height: u16) -> (Vec<PositionedGlyph<'_>>, u16) {
        let text_nfc = text.nfc().filter(|x| !x.is_control()).collect();
        let glyphs = self.glyphs(&text_nfc, height);
        let width = self.calc_width(&glyphs);
        (glyphs, width)
    }

    pub fn draw_text(&self, width: u16, glyphs: &Vec<PositionedGlyph>, fg: &Vec<u8> ,bg: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {

        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw( |x,y,v| {

                    let x = x as i16 + bb.min.x as i16;
                    let y = y as i16 + bb.min.y as i16;

                    let arrpos = (y*width as i16 + x) as usize * 4;
                    if arrpos < bg.len() {

                        for i in 0..3 {
                            bg[arrpos+i] = combine_comp(fg[arrpos+i], bg[arrpos+i], v);
                        }
                    }
                })
            }
        }

        Ok(())
    }
    
}

fn scale(height: u16) -> Scale {
    Scale{x: height as f32, y: height as f32}
}

fn combine_comp(a: u8, b: u8, factor: f32) -> u8 {
    (a as f32 * factor + b as f32 * (1.0 - factor)) as _
}
