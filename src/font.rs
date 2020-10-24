
use crate::draw::{Color, Drawable, DrawableSet};
use crate::utils::find_human_readable;

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
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    pitch: u16,
    advx: u16,
}

#[derive(Debug)]
enum FontError {
    NonScalable(String)
}
impl Error for FontError {}
impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NonScalable(s) => write!(f, "Font {} is not scalable", s)
        }
    }
}


/// TODO subpixel handling
impl Font {
    pub fn new(name: &str, fu: &FontUtils) -> Result<Self, Box<dyn Error>> {
        let fontpath = fu.fc.find(name, None).unwrap().path;

        let face = fu.lib.new_face(fontpath,0)?;

        if !face.is_scalable() {
            return Err(Box::new(FontError::NonScalable(name.to_string())));
        }

        Ok( Self {face, glyphs: HashMap::new(), baseline: HashMap::new()} )
    }

    /// Get distance from top to baseline, or calculate it based on common characters
    fn baseline(&mut self, height: u16) -> u16 {
        let calcdesc = match self.baseline.get(&height) {
            Some(_) => None,
            None => Some({

                self.face.set_pixel_sizes(0, height as _).unwrap();

                "gjpqy".chars().map( |ch| {
                    self.face.load_char(ch as _, LoadFlag::RENDER | LoadFlag::TARGET_LCD).unwrap();
                    
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

                self.face.set_pixel_sizes(0, height as _).unwrap();
                self.face.load_char(ch as _, LoadFlag::RENDER | LoadFlag::TARGET_LCD).unwrap();

                let ftglyph = self.face.glyph();

                Some(Glyph {
                    bitmap: Vec::from(ftglyph.bitmap().buffer()),
                    x: ftglyph.bitmap_left() as i16,
                    y: baseline as i16 - (ftglyph.bitmap_top()) as i16,
                    advx: (ftglyph.advance().x / 64) as u16,
                    w: ftglyph.bitmap().width() as u16 / 3,
                    h: ftglyph.bitmap().rows() as u16,
                    pitch: ftglyph.bitmap().pitch() as u16,
                })
            }
        };
        
        match newglyph {
            Some(x) => {self.glyphs.entry((ch,height)).or_insert(x)},
            None => self.glyphs.get(&(ch, height)).unwrap()
        }

    }

    pub fn width(&mut self, text: &String, height: u16) -> u16 {
        let text_nfc = text.nfc().formatted(None).map(|x| x.0).collect::<String>();
        
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
        ds: &DrawableSet
        ) -> Result<Vec<u8>, Box<dyn Error>> 
    {
        let mut v = ds.background.image(x as i16,y as i16,width,height,maxheight);

        let fchars = text.nfc().formatted(Some(ds)).collect::<Vec<_>>();
        let mut cursor = 0;

        let value = find_human_readable(fchars.iter().map(|x| x.0));
        let fg = ds.value_appearance(value);
        
        for (ch, fgc, bgc) in fchars.iter() {
            let glyph = self.glyph(*ch, height);

            for iy in 0..(glyph.h) {
                for ix in 0..(glyph.w) {

                    let x = x as usize;
                    let y = y as usize;
                    let ix = ix as usize;
                    let iy = iy as usize;
                    let gx = glyph.x as usize;
                    let gy = glyph.y as usize;
                    let cur = cursor as usize;
                    let w = width as usize;

                    let bgindex = ((iy+gy)*w+ix+gx+cur) as usize;

                    let px = (x+ix+gx+cur) as i16;
                    let py = (y+iy+gy) as i16;
                    
                    let fgpix = fg.unwrap_or(fgc).pixel(px, py, maxheight);
                    let bgpix = bgc.pixel(px, py, maxheight);
                    
                    let factor = glyph.pixel(ix as u16, iy as u16);

                    for i in 0..3 {
                        let color = &bgpix.mix(&fgpix, factor.get(i) as f32 / 255.0);
                        if v.len() > bgindex*4+i {
                        v[bgindex*4+i] = color.get(i);}
                        else {
                            eprintln!("x{} y{} ix{} iy{} gx{} gy{} cur{} w{}", x, y,ix,iy,gx,gy,cur,w);
                        }
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
    ds: Option<&'a DrawableSet>,
    fg: Drawable,
    bg: Drawable
}

impl<'a, T> std::iter::Iterator for FormattedTextIter<'a, T> 
    where T: std::iter::Iterator<Item = char>
{
    type Item = (char, Drawable, Drawable);

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
                    
                    if let Some(dset) = self.ds {
                        let sgr = params.next().unwrap_or(0);
                        let (d, isbackground) = dset.sgrcolor(sgr, params.collect());

                        if isbackground {
                            self.bg = d;
                        }
                        else {self.fg = d};
                    }
                }
                else if !first.is_control() {
                    return Some((first, self.fg.clone(), self.bg.clone()));
                }
            }
            else {
                return None;
            }
        }
    }
}

pub trait Formatted<T: std::iter::Iterator<Item = char>> {
    fn formatted<'a>(&'a mut self, ds: Option<&'a DrawableSet>) -> FormattedTextIter<'a, T>;
}

impl<T> Formatted<T> for T 
    where T: std::iter::Iterator<Item = char> {
    fn formatted<'a>(&'a mut self, ds: Option<&'a DrawableSet>) -> FormattedTextIter<'a, T> {
        
        let (fg, bg) = match ds {
            Some(d) => (d.foreground.clone(), d.background.clone()),
            None => (Drawable::from("#FFFFFF".to_string()), Drawable::from("#FFFFFF".to_string()))
        };
        FormattedTextIter { chars: self, ds, fg, bg}
    }
}

impl Glyph {
    fn pixel(&self, x: u16, y: u16) -> Color {
        let pos = (y*self.pitch+x*3) as usize;
        let rgb = self.bitmap.get(pos..(pos+3)).unwrap();
        let avg = (rgb[0] as u16 + rgb[1] as u16 + rgb[2] as u16 / 3 as u16) as u8;
        Color::new(rgb[0], rgb[1], rgb[2], avg)
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
