
use crate::draw::{Color, Drawable, DrawableSet, scale};
use crate::utils::find_human_readable;

use std::error::Error;
use std::collections::HashMap;

use cairo::XCBSurface;

use unicode_normalization::UnicodeNormalization;


/// An object representing character, and, in the future, images etc.
#[derive(Eq, PartialEq, Hash, Clone)]
pub enum CharObj {
    Char(char)
}

/*struct Font {
    faces: Vec<freetype::Face>,
    baselines: Vec<HashMap<u16,u16>>,
    glyphs: HashMap<(CharObj, u16), Glyph>,

    // Dealing with glyphs array directly for checking existence is quite problematic
    glyph_existence: HashMap<(CharObj, u16), ()>
}*/

/*pub struct Renderer {
    surface: XCBSurface
    //fonts: HashMap<String, Font>
}*/

pub struct Glyph {
    bitmap: Vec<u8>,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    pitch: u16,
    advx: u16,
}

/*impl Font {
    fn find_glyph(&self, ch: char, height: u16) -> (usize, u32) {
        self.faces.iter().enumerate().fold(None, |acc,(cid, face)| {
            if let Some(_) = acc {
                acc
            }
            else {
                let g = face.get_char_index(ch as usize);
                if g != 0 {
                    Some((cid, g))
                }
                else {None}
            }
        }).unwrap_or((0,0))
    }

    fn glyph(&mut self, chobj: CharObj, height: u16) -> &Glyph {
        
        let mut exists = true;
        self.glyph_existence.entry((chobj.clone(), height)).or_insert_with(|| {exists = false; ()});

        if exists {
            self.glyphs.get(&(chobj, height)).unwrap()
        } 
        else {
            let x = match chobj {
                CharObj::Char(ch) => {
                    
                    let (id, glyph) = self.find_glyph(ch, height);
                    let face = self.faces.get_mut(id).unwrap();

                    if face.is_scalable() {
                        face.set_pixel_sizes(0, height as _).unwrap();
                    }
                    else {
                        unsafe {
                            let f = face.raw_mut();
                            let i = (0..f.num_fixed_sizes).min_by_key(|x| 
                                ((*f.available_sizes.offset(*x as isize)).height - height as i16).abs()
                            ).unwrap_or(0);

                            FT_Select_Size(f, i);
                        }
                    }

                    let descend = self.baselines.get_mut(id).unwrap().entry(height).or_insert_with(|| {

                        "gjpqy".chars().map( |ch| {
                            face.load_char(ch as _, LoadFlag::RENDER | LoadFlag::TARGET_LCD | LoadFlag::COLOR).unwrap();
                            
                            let ftglyph = face.glyph();
                            ftglyph.bitmap().rows() - ftglyph.bitmap_top()

                        }).fold(0, |acc, x| acc.max(x)) as u16
                    });

                    let baseline = height - *descend;

                    face.load_glyph(glyph, LoadFlag::RENDER | LoadFlag::TARGET_LCD | LoadFlag::COLOR).unwrap();
                    let ftglyph = face.glyph();

                    if face.is_scalable() {
                        // Vector font
                        
                        let bm = ftglyph.bitmap();
                        let buf = bm.buffer();
                        let mut bitmap = Vec::with_capacity(buf.len()*4/3);

                        for i in 0..(bm.pitch() * bm.rows() / 3) {
                            let i = i as usize;
                            let (r,g,b) = (buf[3*i], buf[3*i+1], buf[3*i+2]);
                            bitmap.extend(&[r,g,b,((r as u16 + g as u16 + b as u16)/3u16) as u8]);
                        }

                        let x = Glyph {
                            bitmap,
                            x: ftglyph.bitmap_left() as i16,
                            y: baseline as i16 - (ftglyph.bitmap_top()) as i16,
                            advx: (ftglyph.advance().x / 64) as u16,
                            w: ftglyph.bitmap().width() as u16 / 3,
                            h: ftglyph.bitmap().rows() as u16,
                            pitch: ftglyph.bitmap().pitch() as u16 /3 * 4,
                        };
                        //dbg!(x.x,x.y,x.advx,x.w,x.h,x.pitch);
                        x
                    }
                    else {
                        // Bitmap font/Emoji

                        let w =  height as i32 * ftglyph.bitmap().width() / ftglyph.bitmap().rows();
                        let bitmap = scale(
                            &Vec::from(ftglyph.bitmap().buffer()), ftglyph.bitmap().pitch() as usize,
                            ftglyph.bitmap().width() as usize, ftglyph.bitmap().rows() as usize,
                            w as usize, height as usize
                        );
                        
                        bitmap.iter().filter(|x| **x != 0).for_each(|x| eprint!("{}\t",x));
                        if let Ok(freetype::bitmap::PixelMode::Bgra) = ftglyph.bitmap().pixel_mode() {
                            eprint!("xd");
                        }
                        //dbg!(w,height, ftglyph.bitmap().width(), ftglyph.bitmap().rows());
                        Glyph {
                            bitmap,
                            x: 5,
                            y: 0 as i16,
                            advx: w as u16 * 3,
                            w: w as u16,
                            h: height,
                            pitch: w as u16
                        }
                    }
                }
            };
            self.glyphs.entry((chobj, height)).or_insert(x)
        }
    }
}*/

/*impl Renderer {
    //pub fn new(fonts: HashMap<String, Vec<String>>, fu: &FontUtils) -> Renderer {
    //pub fn new() -> Renderer {

        /*Self { fonts: fonts.iter().map(|(k,v)| {
            let faces = v.iter()
                .filter_map(|name| 
                    fu.fc.find(name, None)
                        .and_then(|x| fu.lib.new_face(x.path,0).ok())
                ).collect::<Vec<_>>();

            let baselines = vec!(HashMap::new(); faces.len());
            
            (k.clone(), Font {faces, baselines, glyphs: HashMap::new(), glyph_existence: HashMap::new()})
        }).collect()}*/
    //}


    // / Get a glyph for specified character, create one if unavailable
    /*pub fn glyph(&mut self, ch: CharObj, font: &String, height: u16) -> &Glyph {
        self.fonts.get_mut(font).unwrap().glyph(ch, height)
    }
    */
    /*pub fn width(&mut self, text: &String, font: &String, height: u16) -> u16 {
        text.nfc().formatted(None).fold(0, |acc, (ch, _, _)| {
            acc + (self.glyph(ch, font, height).advx) as u16
        })
    }*/

    /*pub fn draw_text(&mut self, 
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        maxheight: u16,
        text: &String,
        font: &String,
        ds: &DrawableSet
        ) -> Result<Vec<u8>, Box<dyn Error>> 
    {
        /*let mut v = ds.background.image(x as i16,y as i16,width,height,maxheight);

        let fchars = text.nfc().formatted(Some(ds)).collect::<Vec<_>>();
        let mut cursor = 0;

        let value = find_human_readable(fchars.iter().filter_map(|x| 
            if let CharObj::Char(c) = x.0 {Some(c)} else {None}
        ));
        let fg = ds.value_appearance(value);

        
        for (ch, fgc, bgc) in fchars.iter() {
            let glyph = self.glyph(ch.clone(), font, height);


            let x = x as isize;
            let y = y as isize;
            let gx = glyph.x as isize;
            let gy = glyph.y as isize;
            let cur = cursor as isize;
            let w = width as isize;

            let px = (x+gx+cur) as i16;
            let py = (y+gy) as i16;

            let fgimg = fg.unwrap_or(fgc).image(px, py, glyph.w, glyph.h, maxheight);
            let bgimg = bgc.image(px, py, glyph.w, glyph.h, maxheight);

            for iy in 0..(glyph.h) {
                for ix in 0..(glyph.w) {
                    let ix = ix as isize;
                    let iy = iy as isize;
                    let bgindex = ((iy+gy)*w+ix+gx+cur) as usize;
                    if bgindex > 0xffffffff {
                        continue;
                    }

                    let pos = (iy*(glyph.w as isize)+ix) as usize *4;
                    let bgpix = Color::new(bgimg[pos+2], bgimg[pos+1], bgimg[pos],bgimg[pos+3]);
                    let fgpix = Color::new(fgimg[pos+2], fgimg[pos+1], fgimg[pos],fgimg[pos+3]);
                    
                    let factor = glyph.pixel(ix as u16, iy as u16);

                    for i in 0..3 {
                        let color = &bgpix.mix(&fgpix, factor.get(i) as f32 / 255.0);
                        if v.len() > bgindex*4+i {
                            v[bgindex*4+i] = color.get(i);
                        }
                    }
                }
            }
            cursor += glyph.advx;
        }
        
        Ok(v)*/
    }*/
    
}*/

pub struct FormattedTextIter<'a, T: std::iter::Iterator<Item = char>> {
    chars: &'a mut T,
    ds: Option<&'a DrawableSet>,
    fg: Drawable,
    bg: Drawable
}

impl<'a, T> std::iter::Iterator for FormattedTextIter<'a, T> 
    where T: std::iter::Iterator<Item = char>
{
    type Item = (CharObj, Drawable, Drawable);

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
                    return Some((CharObj::Char(first), self.fg.clone(), self.bg.clone()));
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
        let pos = (y*self.pitch+x*4) as usize;
        let rgb = self.bitmap.get(pos..(pos+4)).unwrap_or(&[0,0,0,0]);
        //let avg = (rgb[0] as u16 + rgb[1] as u16 + rgb[2] as u16 / 3 as u16) as u8;
        Color::new(rgb[0], rgb[1], rgb[2], rgb[3])
    }
}

/*impl FontUtils {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            fc: match fontconfig::Fontconfig::new() {
                Some(x) => x,
                None => {panic!("Failed to initialize Fontconfig")} // TODO result
            },
            lib: freetype::Library::init()?
        })
    }
}*/
