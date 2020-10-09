
use crate::draw::{Color, Drawable};
use crate::utils::{mul_comp, mix_comp};

use std::error::Error;
use std::collections::HashMap;

use fontconfig;
use freetype;
use freetype::face::LoadFlag;
use unicode_normalization::UnicodeNormalization;


pub struct FontUtils {
    fc: fontconfig::Fontconfig,
    lib: freetype::Library
}

pub struct Font {
    face: freetype::Face,
    baseline: HashMap<u16, u16>,
    glyphs: HashMap<(char, u16), Glyph>
}

pub struct Glyph {
    bitmap: Vec<u8>,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    advx: u16,
    advy: u16
}

/// TODO subpixel handling
impl Font {
    pub fn new(name: &str, fu: &FontUtils) -> Result<Self, Box<dyn Error>> {
        let fontpath = fu.fc.find(name, None).unwrap().path;

        let face = fu.lib.new_face(fontpath,0)?;

        Ok( Self {face, glyphs: HashMap::new(), baseline: HashMap::new()} )
    }

    /// Get distance from top to baseline, or calculate it based on common characters
    fn baseline(&mut self, height: u16) -> u16 {
        let calcdesc = match self.baseline.get(&height) {
            Some(_) => None,
            None => Some({

                self.face.set_pixel_sizes(0, height as _).expect("Failed to set font size");

                "gjpqy".chars().map( |ch| {
                    self.face.load_char(ch as _, LoadFlag::RENDER).unwrap();
                    
                    let ftglyph = self.face.glyph();
                    ftglyph.bitmap().rows() - ftglyph.bitmap_top()

                }).fold(0, |acc, x| acc.max(x)) as u16
            })
        };
        
        let descend = match calcdesc {
            Some(x) => *self.baseline.entry(height).or_insert(x),
            None => *self.baseline.get(&height).unwrap()
        };

        height - descend
    }

    /// Get a glyph for specified character, create one if unavailable
    pub fn glyph(&mut self, ch: char, height: u16) -> &Glyph {

        let baseline = self.baseline(height);

        let newglyph = match self.glyphs.get(&(ch, height)) {
            Some(_) => None,
            None => {

                self.face.set_pixel_sizes(0, height as _).expect("Failed to set font size");
                self.face.load_char(ch as _, LoadFlag::RENDER).unwrap();

                let ftglyph = self.face.glyph();

                Some(Glyph {
                    bitmap: Vec::from(ftglyph.bitmap().buffer()),
                    x: (ftglyph.bitmap_left()) as u16,
                    y: baseline - (ftglyph.bitmap_top()) as u16,
                    advx: (ftglyph.advance().x / 64) as u16,
                    advy: (ftglyph.advance().y / 64) as u16,
                    w: ftglyph.bitmap().width() as u16,
                    h: ftglyph.bitmap().rows() as u16,
                })
            }
        };
        
        match newglyph {
            Some(x) => {self.glyphs.entry((ch,height)).or_insert(x)},
            None => self.glyphs.get(&(ch, height)).unwrap()
        }

    }

    pub fn width(&mut self, text: &String, height: u16) -> u16 {
        let text_nfc = text.nfc().formatted().map(|x| x.0).collect::<String>();
        
        text_nfc.chars().fold(0, |acc, ch| {
            acc + (self.glyph(ch, height).advx) as u16
        })
    }

    pub fn draw_text(&mut self, 
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        maxheight: u16,
        text: &String,
        fg: &Drawable,
        bg: &Drawable
        ) -> Result<Vec<u8>, Box<dyn Error>> 
    {
        let mut v = bg.image(x as i16,y as i16,width,height,maxheight);

        let fchars = text.nfc().formatted().collect::<Vec<_>>();
        let mut cursor = 0;
        
        for (ch, fgc, bgc) in fchars.iter() {
            let glyph = self.glyph(*ch, height);

            for iy in 0..(glyph.h) {
                for ix in 0..(glyph.w) {
                    let bgindex = ((iy+glyph.y)*width+ix+glyph.x+cursor) as usize;

                    let px = (x+ix+glyph.x+cursor) as i16;
                    let py = (y+iy+glyph.y) as i16;
                    
                    let fgpix = fg.pixel(px, py, maxheight).mul(fgc);
                    let bgpix = bg.pixel(px, py, maxheight).mul(bgc);
                    
                    let factor =  (glyph.bitmap[(iy*glyph.w+ix) as usize] as f32) / 255.0;
                    let color = &bgpix.mix(&fgpix, factor);

                    for i in 0..3 {
                        v[bgindex*4+i] = color.get(i);
                    }
                }
            }
            cursor += glyph.advx;
        }
        
        Ok(v)
    }
    
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

impl FontUtils {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            fc: match fontconfig::Fontconfig::new() {
                Some(x) => x,
                None => {panic!("Failed to initialize Fontconfig")} // TODO result
            },
            lib: freetype::Library::init()?
        })
    }
}
