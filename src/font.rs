
use fontconfig;
use rusttype;
use rusttype::{point, Scale};
use x11rb::protocol::xproto::{Rectangle, ImageFormat, CreateGCAux, GX};

use crate::window::{XConnection, Window};
use std::error::Error;


pub type FontConfig = fontconfig::Fontconfig;

pub struct Font<'a> {
    font: rusttype::Font<'a>,
    scale: Scale,

    max_ascent: f32,
    max_descent: f32
}

impl Font<'_> {
    pub fn new(name: &str, maxheight: u16, fc: &FontConfig) -> Result<Self, Box<dyn Error>> {
        let fontpath = fc.find(name, None).unwrap().path;

        let font = rusttype::Font::try_from_vec(std::fs::read(fontpath)?).unwrap();
        
        let scale = rusttype::Scale{x: maxheight as f32, y: maxheight as f32};
        let vmetrics = font.v_metrics(scale);

        Ok( Self {font, scale, max_ascent: vmetrics.ascent, max_descent: vmetrics.descent} )
    }

    fn glyphs(&self, text: &str) -> Vec<rusttype::PositionedGlyph> {
        self.font.layout(text, self.scale, point(0.0, self.max_ascent)).collect::<Vec<_>>()
    }

    pub fn calc_text_rect(&self, original: Rectangle, text: &str) -> Rectangle {
        let glyphs = self.glyphs(text);
        
        let width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0).ceil() as _;

        let x = original.x;
        let y = original.y;
        let height = (self.max_ascent - self.max_descent) as u16;
        Rectangle {x,y,width , height: height}
    }

    pub fn draw_text<T: XConnection>(&self, text: &str, window: &Window<T>, x: i16, y: i16) -> Result<u16, Box<dyn Error>> {
        
        // Get glyphs and text extents
        let glyphs = self.glyphs(text);

        let rect = self.calc_text_rect(Rectangle{x:0,y:0,width:0,height:0}, text);

        // Draw glyphs to buffer
        let mut data = vec![0u8; (rect.width * rect.height * 4) as _];
        
        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw( |x,y,v| {

                    let x = x as i16 + bb.min.x as i16;
                    let y = y as i16 + bb.min.y as i16;

                    let arrpos = (y*rect.width as i16 + x) as usize * 4;
                    if arrpos < data.len() {

                        for _ in 0..3 {
                            data[arrpos] = (v*255.) as _;
                        }
                    }
                })
            }
        }

        // Draw image to window
        let gc = window.conn.generate_id()?;
        window.conn.create_gc(gc, window.window, &CreateGCAux::new())?;

        //crate::window::Drawable::from("#FFFF00".to_string()).draw_rect(window, Rectangle{x,y, width: rect.width, height: rect.height})?;
        window.conn.put_image(
            ImageFormat::ZPixmap, 
            window.window, 
            gc, 
            rect.width, 
            rect.height,
            x,
            y,
            0, 
            24, 
            &data)?;
        
        

        window.conn.free_gc(gc)?;

        println!("Text geom: {} {} {} {}", x,y,rect.width, rect.height);

        Ok(rect.width)
    }
    
}
