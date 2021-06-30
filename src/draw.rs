
use crate::window::Window;
use crate::utils::mix_comp;
use crate::props::WidgetPropsCurrent;
use crate::utils::find_human_readable;
use crate::font::{CharObj, Formatted as _};

use cairo::{TextExtents, Pattern, Operator};
use unicode_normalization::UnicodeNormalization;


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

#[derive(Debug)]
pub struct DrawFGInfo {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub fgy: i16,
    pub fgheight: u16,
    pub xb: f64,
    pub yb: f64,
    pub font: String
}

pub struct DrawableSet {
    pub foreground:  Drawable,
    pub background:  Drawable,

    pub black:    Drawable,
    pub red:      Drawable,
    pub green:    Drawable,
    pub yellow:   Drawable,
    pub blue:     Drawable,
    pub magenta:  Drawable,
    pub cyan:     Drawable,
    pub white:    Drawable,

    pub bright_black:    Drawable,
    pub bright_red:      Drawable,
    pub bright_green:    Drawable,
    pub bright_yellow:   Drawable,
    pub bright_blue:     Drawable,
    pub bright_magenta:  Drawable,
    pub bright_cyan:     Drawable,
    pub bright_white:    Drawable,

    pub warn: f64,
    pub critical: f64,
    pub dim: f64
}

impl DrawFGInfo {
    
    pub fn new(window: &Window, x: i16, y: i16, height: u16, border_factor: f32, font: &String, text: &String) -> DrawFGInfo {
        
        let fgheight = (height as f32 * border_factor).ceil() as _;
        let fgy = y + ((height - fgheight) / 2) as i16;
        
        //let width = 10;//renderer.width(text, font, fgheight);
        // TODO another way of processing it (command vectors?)
        let fchars = text.nfc().formatted(None).map(|(ch,_,_)| if let CharObj::Char(c) = ch {c} else {'?'}).collect::<String>();
        let (xb, yb, width, _) = window.get_text_extents(&fchars, font, fgheight as f64);
        
        // width + 1  - more or less prevent from clipping text
        DrawFGInfo {x,y,width: width as u16 + 1,height, fgy,fgheight, xb,yb, font: font.clone()}
    }
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {r,g,b,a}
    }

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

    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn get(&self, i: usize) -> u8 {
        self.array()[i]
    }

    pub fn array(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
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

    /*
    pub fn pixel(&self, _x: i16, y: i16, maxheight: u16) -> Color {
        match self {
            Self::Color(c) => *c,
            Self::VGradient(cv) => {
                let index = ((y as f32)/maxheight as f32) * (cv.len() - 1) as f32;

                let color1: Color = cv[index.floor() as usize];
                let color2: Color = cv[index.ceil() as usize];
                color1.mix(&color2, index.fract())
            }
        }
    }

    pub fn image(&self, x: i16, y: i16, width: u16, height: u16, maxheight: u16) -> Vec<u8> {
        let mut v = Vec::with_capacity((width * height) as usize * 4);
        
        for iy in y..(y+height as i16) {
            for ix in x..(x+width as i16) {
                v.extend(&self.pixel(ix, iy, maxheight).array())
            }
        }

        v
    }
*/


    pub fn draw(&self, window: &Window, mask: Option<CharObj>, x: i16, y: i16, width: u16, height: u16, maxheight: u16) -> f64 {
        
        //let string = string;//CharObj::vec_from(string)
        let c = &window.ctx;

        let norm = |x| (x as f64) / 255.0;

        match self {
            Self::Color(col) => c.set_source_rgba(norm(col.r), norm(col.g), norm(col.b), norm(col.a)),
            Self::VGradient(v) => {
                let src = cairo::LinearGradient::new(0.0, 0.0 as f64, 0.0, maxheight as f64);
                for (c,i) in v.iter().enumerate() {
                    src.add_color_stop_rgba(c as f64 / (v.len()-1) as f64, norm(i.r),norm(i.g),norm(i.b), norm(i.a));
                }
                c.set_source(&src);
            }
        };
        
        let extents = match mask {
            Some(CharObj::Char(ch)) => {
                
                let text = &ch.to_string();
                c.set_operator(Operator::Over);
                
                c.move_to(x as f64, y as f64);
                c.show_text(text);

                //dbg!(ch,x,y,width,height,maxheight);
                c.text_extents(text).x_advance
            }
            None => {
                c.set_operator(Operator::Source);
                c.move_to(x as f64, y as f64);
                c.rectangle(x as f64, y as f64, width as f64, height as f64);
                c.fill();

                width as f64
            }
        };

        extents
        
        /*match self {
            Drawable::Color(c) => {
                let gc = window.conn.generate_id()?;

                window.conn.create_gc(gc, window.window, &CreateGCAux::new().foreground(c.as_xcolor()))?;

                let rect = Rectangle {x,y,width,height};
                window.conn.poly_fill_rectangle(window.window, gc, &[rect])?;
                
                window.conn.flush()?;

                window.conn.free_gc(gc)?;
            }
            _ => draw_image(window, x, y, width, height, 
                            &self.image(x, y, width, height, maxheight))?
        }
        Ok(())*/
    }
}


// TODO put somewhere else
/*fn draw_image(window: &Window, x: i16, y: i16, width: u16, height: u16, data: &Vec<u8>) -> Result<(), Box<dyn Error>> {
    
    let gc = window.conn.generate_id()?;
    window.conn.create_gc(gc, window.window, &CreateGCAux::new())?;

    window.conn.put_image(ImageFormat::ZPixmap, window.window, gc, 
        width, height, x, y, 0, window.depth, &data)?;
    
    window.conn.free_gc(gc)?;
    Ok(())
}*/


impl DrawableSet {

    pub fn from(props: &WidgetPropsCurrent) -> Self {
        Self {
            foreground: props.foreground.clone(),
            background: props.background.clone(),
            
            black: props.black.clone(),
            red: props.red.clone(),
            green: props.green.clone(),
            yellow: props.yellow.clone(),
            blue: props.blue.clone(),
            magenta: props.magenta.clone(),
            cyan: props.cyan.clone(),
            white: props.white.clone(),
            
            bright_black: props.bright_black.clone(),
            bright_red: props.bright_red.clone(),
            bright_green: props.bright_green.clone(),
            bright_yellow: props.bright_yellow.clone(),
            bright_blue: props.bright_blue.clone(),
            bright_magenta: props.bright_magenta.clone(),
            bright_cyan: props.bright_cyan.clone(),
            bright_white: props.bright_white.clone(),

            warn: props.warn,
            critical: props.critical,
            dim: props.dim
        }
    }

    pub fn sgrcolor(&self, n: u32, params: Vec<u32>) -> (Drawable, bool) {

        let isbackground = match (n/10) % 2 {
            0 => true,
            _ => false
        };

        let drawable = match n % 10 {
            8 => match params.get(0) {
                // True color
                Some(2) => match params.get(1..4) {
                    Some(x) => {let r = x[0] as _; let g = x[1] as _; let b = x[2] as _;
                                Drawable::Color(Color{r,g,b,a:255})},
                    None => self.basecolor(39, isbackground)
                }
                // 256 color palette
                Some(5) => match params.get(1) {
                    Some(x) => {
                        if x < &8 {
                            self.basecolor(x+30, isbackground)
                        }
                        else if x < &16 {
                            self.basecolor(x+90, isbackground)
                        }
                        else if x < &232 {
                            let r = (x-16) / 36;
                            let g = ((x-16) / 6) % 6;
                            let b = (x-16) % 6;
                            
                            let (r,g,b) = ((r*256/6) as _, (g*256/6) as _, (b*256/6) as _);
                            Drawable::Color(Color{r,g,b,a:255})
                        }
                        else {
                            let v = ((x - 232) * 256 / 24) as u8;
                            Drawable::Color(Color{r:v,g:v,b:v,a:255})
                        }
                    }
                    None => self.basecolor(39, isbackground)
                },
                _ => self.basecolor(39, isbackground)
            }
            // 16 color palette
            _ => self.basecolor(n, isbackground)
        };

        (drawable, isbackground)
    }

    pub fn basecolor(&self, n: u32, isbackground: bool) -> Drawable {

        match n {
            30 => self.black.clone(),
            31 => self.red.clone(),
            32 => self.green.clone(),
            33 => self.yellow.clone(),
            34 => self.blue.clone(),
            35 => self.magenta.clone(),
            36 => self.cyan.clone(),
            37 => self.white.clone(),
            
            90 => self.bright_black.clone(),
            91 => self.bright_red.clone(),
            92 => self.bright_green.clone(),
            93 => self.bright_yellow.clone(),
            94 => self.bright_blue.clone(),
            95 => self.bright_magenta.clone(),
            96 => self.bright_cyan.clone(),
            97 => self.bright_white.clone(),

            _ => if isbackground {self.background.clone()} else {self.foreground.clone()}
        }
    }

    pub fn value_appearance(&self, value: Option<f64>) -> Option<&Drawable> {
        match value {
            None => None,
            Some(x) => if x >= self.critical {
                Some(&self.red)
            }
            else if x >= self.warn {
                Some(&self.yellow)
            }
            else if x <= self.dim {
                Some(&self.bright_black)
            }
            else {None}
        }
    }

    pub fn draw_widget(
        &self,
        window: &Window, 
        info: &DrawFGInfo, 
        offset: i16,
        width_max: u16, 
        text: &String)
    {
        let i = info;

        let fchars = text.nfc().formatted(Some(self)).collect::<Vec<_>>();
        let mut cursor = 0;

        // Change foreground color if the value crosses dim/warn/critical treshold
        let value = find_human_readable(fchars.iter().filter_map(|x| 
            if let CharObj::Char(c) = x.0 {Some(c)} else {None}
        ));
        let fg = self.value_appearance(value);

        //for (ch, fgc, bgc) in fchars.iter() {
        //    let fg = fg.unwrap_or(fgc);
        //}

        // Text
        let lrborder = (width_max - i.width) as i16 / 2;
        let fgx = offset + i.x + lrborder;
        //let bg = renderer.draw_text(fgx as _,i.fgy as _,i.width, i.fgheight, i.height, &text, &i.font, &ds)?;
        //self.foreground.draw(window, None, offset + fgx, i.fgy, i.width, i.fgheight, i.height);//&bg);

        let mut x = fgx as f64;
        dbg!(x, i.x, width_max, i.width);

        window.ctx.select_font_face(&i.font[..], cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        window.set_font_height_px(&i.font, i.fgheight as f64);
        dbg!(i);
        
        // todo - var bg
        self.background.draw(window, None, fgx, i.fgy as i16, i.width, i.fgheight, i.height);
        for (ch, fgc, bgc) in fchars {
            
            x += fg.unwrap_or(&fgc).draw(window, Some(ch), x as i16, i.fgy - i.yb as i16, i.width, i.fgheight, i.height);
        }

        // Top and bottom borders
        self.background.draw(window, None, offset + i.x, i.y, width_max, (i.fgy - i.y) as _, i.height);
        self.background.draw(window, None, offset + i.x, i.fgy+i.fgheight as i16, width_max, (i.height - i.fgy as u16 - i.fgheight) as _, i.height);
        
        
        // Left and right borders
        self.background.draw(window, None, offset + i.x, i.fgy, lrborder as _, i.fgheight, i.height);
        self.background.draw(window, None, fgx + i.width as i16, i.fgy, lrborder as _, i.fgheight, i.height);
    }
}


fn rescale_coord(x: usize, old: usize, new: usize) -> (usize, f32, f32) {
    let o = (x as f32) * (old as f32) / (new as f32);
    (o.floor() as usize, o.fract(), 1.0 - o.fract())
}


pub fn scale(original: &Vec<u8>, pitch: usize, oldw: usize, oldh: usize, neww: usize, newh: usize) -> Vec<u8> {
    let bpp = original.len() / pitch / oldh;
    let mut v = Vec::with_capacity(neww * newh * bpp);

    let idx = |x,y| pitch*y+x;
    let o = |(w,i),b| ((*original.get(i*bpp+b).unwrap_or(&0) as f32) * w) as u8;

    for y in 0..newh {
        let (yo, yl1, yl2) = rescale_coord(y, oldh, newh);

        for x in 0..neww {
            let (xo, xl1, xl2) = rescale_coord(x, oldw, neww);

            let weights = [(xl1*yl1, idx(xo+0, yo+0)), 
                           (xl2*yl1, idx(xo+1, yo+0)),
                           (xl1*yl2, idx(xo+0, yo+1)),
                           (xl2*yl2, idx(xo+1, yo+1))];
            for b in 0..bpp {
                v.push(weights.iter().map(|x| o(*x,b)).sum());
            }
        }  
    }
    v
}
