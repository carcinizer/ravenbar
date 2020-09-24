
use crate::draw::Color;
use crate::utils::{mul_comp, mix_comp};

use std::error::Error;

use fontconfig;
use rusttype;
use rusttype::{point, PositionedGlyph, Scale};
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
        let text_nfc = text.nfc().formatted().map(|x| x.0).collect::<String>();
        let glyphs = self.glyphs(&text_nfc, height);
        let width = self.calc_width(&glyphs);
        (glyphs, width)
    }

    pub fn draw_text(&self, width: u16, height: u16, text: &String, fg: &Vec<u8> ,bg: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {

        let fchars = text.nfc().formatted().collect::<Vec<_>>();
        
        let plaintext = fchars.iter().map(|x| x.0).collect::<String>();
        let glyphs = self.glyphs(&plaintext, height);

        for (g, (fgc,bgc)) in glyphs.iter().zip(fchars.iter().map(|x| (x.1,x.2))) {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw( |x,y,v| {

                    let x = x as i16 + bb.min.x as i16;
                    let y = y as i16 + bb.min.y as i16;

                    let arrpos = (y*width as i16 + x) as usize * 4;
                    if arrpos < bg.len() {

                        for i in 0..3 {
                            let fgformat = mul_comp(fg[arrpos+i], fgc.get(i));
                            let bgformat = mul_comp(bg[arrpos+i], bgc.get(i));
                            bg[arrpos+i] = mix_comp(bgformat, fgformat, v);
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

pub struct FormattedTextIter<'a, T: std::iter::Iterator<Item = char>> {
    chars: &'a mut T,
    fg: Color,
    bg: Color
}

impl<'a, T> std::iter::Iterator for FormattedTextIter<'a, T> 
    where T: std::iter::Iterator<Item = char>
{
    type Item = (char, Color, Color);

    fn next(&mut self) -> Option<Self::Item> {
            
        loop {
            let first_opt = self.chars.next();
            if let Some(first) = first_opt {
                
                // Escape code && CSI
                if first == '\x1b' && self.chars.next() == Some('[') {

                    let sgrstring = self.chars
                        .take_while(|x| !x.is_alphabetic())
                        .collect::<String>();

                    let mut params = sgrstring
                        .split(';')
                        .map(|x| x.parse::<u32>().unwrap_or(0));
                    
                    let sgr = params.next().unwrap_or(0);
                    let color = Color::from_sgr(sgr%10, &params.collect());

                    let (fg,bg) = match sgr/10 {
                        3   => (color, self.bg),
                        9   => (color.bright(), self.bg),
                        4   => (self.fg, color),
                        10  => (self.fg, color.bright()),

                        0 => if sgr == 0 {
                                (Color::white(), Color::white())
                             } 
                             else {
                                (self.fg, self.bg)
                             },

                        _ => (self.fg, self.bg)
                    };
                    self.fg = fg;
                    self.bg = bg;
                }
                else if !first.is_control() {
                    return Some((first, self.fg, self.bg));
                }
            }
            else {
                return None;
            }
        }
    }
}

pub trait Formatted<T: std::iter::Iterator<Item = char>> {
    fn formatted(&mut self) -> FormattedTextIter<'_, T>;
}

impl<T> Formatted<T> for T 
    where T: std::iter::Iterator<Item = char> {
    fn formatted(&mut self) -> FormattedTextIter<'_, T> {
        FormattedTextIter { chars: self, fg: Color::white(), bg: Color::white() }
    }
}

