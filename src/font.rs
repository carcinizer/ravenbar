
use crate::draw::{Color, Drawable, DrawableSet, scale};
use crate::utils::find_human_readable;
use crate::window::Window;

use std::error::Error;
use std::collections::HashMap;
use std::cell::RefCell;

use cairo_sys;
use cairo::{Glyph, ScaledFont};


use unicode_normalization::UnicodeNormalization;


/// An object representing character, and, in the future, images etc.
pub enum GlyphObj {
    Str(usize, u16, Vec<Glyph>)
}

pub struct Font {
    faces: Vec<(freetype::Face, cairo::FontFace)>,
    scaledfonts: RefCell<HashMap<(usize, u16), cairo::ScaledFont>>,
    // Fonts suitable for a given character
    suitablefonts: RefCell<HashMap<char, usize>>,

    // Font heights in pixels at 100 pts
    heights100: Vec<f64>
    
}

pub struct GlyphSet {
    pub glyphs: GlyphObj,
    pub fg: Drawable,
    pub bg: Drawable,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub xb: f64,
    pub yb: f64,
}

impl Font {
    pub fn new(window: &Window, fonts: &Vec<String>) -> Self {

        let faces = fonts.iter().filter_map(|name| 
            window.fc.find(name, None)
                .and_then(|x| window.ft.new_face(x.path,0).ok())
                .and_then(|mut x| unsafe {
                    let cft = cairo::FontFace::create_from_ft(x.raw_mut() as *mut _ as cairo::freetype::freetype::FT_Face).unwrap();
                    Some((x, cft))
            })
        ).collect::<Vec<_>>();

        let heights100 = faces.iter().map(|(_ftface, cairoface)| {
            window.ctx.set_font_face(cairoface);
            window.ctx.set_font_size(100.0);
            window.ctx.text_extents("AŹqpdyj🙂").and_then(|x| Ok(x.height)).unwrap_or(100.0) // some common letters
        }).collect();

        &heights100;

        Self { faces, scaledfonts: RefCell::default(), suitablefonts: RefCell::default(), heights100 }
    }

    /// Find a font that has a glyph for a given character
    fn find_suitable_char_font_uncached(&self, ch: char) -> usize {
        self.faces.iter().enumerate().fold(None, |acc,(fid, face)| {
            if let Some(_) = acc {
                acc
            }
            else {
                let g = face.0.get_char_index(ch as usize);
                if g != 0 {
                    Some(fid)
                }
                else {None}
            }
        }).unwrap_or(0)
    }

    /// Find a font that has a glyph for a given character, cached
    pub fn find_suitable_char_font(&self, ch: char) -> usize {
        let mut suitablefonts = self.suitablefonts.borrow_mut();
        suitablefonts.entry(ch).or_insert_with(|| self.find_suitable_char_font_uncached(ch)).clone()
    }

    fn get_scaled_font_uncached(&self, font: usize, height: u16) -> ScaledFont {
        let height_pt = 100.0 * height as f64 / self.heights100[font];
        let mat = cairo::Matrix::new(height_pt,0.0,0.0,height_pt,0.0,0.0);
        ScaledFont::new(&self.faces[font].1, &mat, &cairo::Matrix::identity(), &cairo::FontOptions::new().unwrap()).unwrap()
    }

    pub fn with_scaled_font<T,F>(&self, font: usize, height: u16, f: F) -> T 
    where F: Fn(&ScaledFont) -> T 
    {
        let mut sfs = self.scaledfonts.borrow_mut();
        let font = sfs.entry((font, height)).or_insert_with(|| self.get_scaled_font_uncached(font, height));
        f(font)
    }
}


pub struct FormattedTextIter<'a, T: std::iter::Iterator<Item = char>> {
    chars: &'a mut T,
    font: &'a Font,
    ds: &'a DrawableSet,
    window: &'a Window,
    buffer: String,
    buffont: Option<usize>,
    x: f64,
    y: f64,
    height: u16,
    fg: Drawable,
    bg: Drawable
}

impl<T: std::iter::Iterator<Item = char>> FormattedTextIter<'_, T> {

    fn return_glyph_set(&mut self, ch: Option<char>, font: Option<usize>) -> GlyphSet {

        let buffont = self.buffont.unwrap_or(0);

        let (glyphs, extents) = self.font.with_scaled_font(buffont, self.height, |sfont| {

            self.window.ctx.set_scaled_font(sfont);
            let glyphs = sfont.text_to_glyphs(0.0,0.0,&self.buffer[..]).unwrap_or_default().0;
            let extents = self.window.ctx
                .glyph_extents(&glyphs[..])
                .unwrap_or(cairo::TextExtents {height: 0.0, width: 0.0, x_advance: 0.0, y_advance: 0.0, x_bearing: 0.0, y_bearing: 0.0});

            (glyphs, extents)
        });

        self.buffer = if let Some(c) = ch {c.to_string()} else {String::new()};
        self.buffont = font.or(self.buffont);

        dbg!((self.x, extents.x_advance, extents.width));

        let x = self.x;
        self.x += extents.x_advance;
        let y = self.y;
        self.y += extents.y_advance;

        GlyphSet {
            glyphs: GlyphObj::Str(buffont, self.height, glyphs),
            fg: self.fg.clone(),
            bg: self.bg.clone(),
            x, y,
            width:  extents.width,
            height: extents.height,
            xb: extents.x_bearing,
            yb: extents.y_bearing
        }
    }
}

impl<'a, T> std::iter::Iterator for FormattedTextIter<'a, T> 
    where T: std::iter::Iterator<Item = char>
{
    type Item = GlyphSet;

    fn next(&mut self) -> Option<Self::Item> {

        loop {
            let first_opt = self.chars.next();
            if let Some(first) = first_opt {
                
                let mut appearance_changed = false;
                let mut char_after = None;
                let mut font_after = None;

                // Escape code && CSI
                if first == '\x1b' && self.chars.next() == Some('[') {

                    let sgrstring = self.chars
                        .take_while(|x| !x.is_alphabetic())
                        .collect::<String>();

                    let mut params = sgrstring
                        .split(';')
                        .map(|x| x.parse::<u32>().unwrap_or(0));
                    
                    let sgr = params.next().unwrap_or(0);
                    let (d, isbackground) = self.ds.sgrcolor(sgr, params.collect());

                    if isbackground {
                        self.bg = d;
                    }
                    else {self.fg = d};
                    
                    appearance_changed = true;
                }
                else if !first.is_control() {

                    let font = self.font.find_suitable_char_font(first);

                    match self.buffont {
                        None => {
                            self.buffer.push(first);
                            self.buffont = Some(font);
                        }
                        Some(buffont) => {
                            if font == buffont {
                                self.buffer.push(first);
                            }
                            else {
                                appearance_changed = true;
                                char_after = Some(first);
                                font_after = Some(font);
                            }
                        }
                    }
                }

                if appearance_changed && (!self.buffer.is_empty() || char_after != None) {
                    return Some(self.return_glyph_set(char_after, font_after));
                }
            }
            else if self.buffer.is_empty() {
                return None;
            }
            else {
                return Some(self.return_glyph_set(None, None));
            }
        }
    }
}

pub trait Formatted<T: std::iter::Iterator<Item = char>> {
    fn formatted<'a>(&'a mut self, window: &'a Window, ds: &'a DrawableSet, font: &'a Font, x: f64, y: f64, height: u16) -> FormattedTextIter<'a, T>;
}

impl<T> Formatted<T> for T 
    where T: std::iter::Iterator<Item = char> {
    fn formatted<'a>(&'a mut self, window: &'a Window, ds: &'a DrawableSet, font: &'a Font, x: f64, y: f64, height: u16) -> FormattedTextIter<'a, T> {
        
        let (fg, bg) = (ds.foreground.clone(), ds.background.clone());
        FormattedTextIter { chars: self, window, font, ds, fg, bg, x, y, height, buffer: String::default(), buffont: None }
    }
}

