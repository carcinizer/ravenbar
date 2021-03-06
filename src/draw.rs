
use crate::window::Window;
use crate::properties::WidgetPropertiesCurrent;
use crate::utils::{Log, find_human_readable};
use crate::font::{GlyphObj, GlyphSet, Font, Formatted as _};

use cairo::{Operator, Glyph};
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

#[derive(Default)]
pub struct DrawFGInfo {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub gsets: Vec<GlyphSet>,
    pub fgy: i16,
    pub fgheight: u16
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
    
    pub fn new(window: &Window, ds: &DrawableSet, x: i16, y: i16, height: u16, border_factor: f32, font: &Font, text: &String) -> DrawFGInfo {
        
        let fgheight = (height as f32 * border_factor).ceil() as _;
        let fgy = y + ((height as u16 - fgheight) / 2) as i16;
        
        let value = find_human_readable(text.chars());

        let gsets = ds.mark_color(value).chars()
            .chain(text.chars())
            .nfc()
            .formatted(window, ds, font, 0.0, fgy as f64, fgheight)
            .collect::<Vec<_>>();
        
        let maxx = gsets.last().and_then(|s| Some(s.x + s.width)).unwrap_or(x as f64);
        let minx = gsets.first().and_then(|s| Some(s.x)).unwrap_or(x as f64);
        let width = (maxx - minx) as u16;

        DrawFGInfo {gsets, fgy, fgheight, x, y, width, height}
    }
}

impl Color {
    pub fn from(s: &str) -> Self {
        if (s.len() != 7 && s.len() != 9) || &s[0..1] != "#" {
            let quote_warn = if s.len() == 0 {&" (have you put color string in quotes)"} else {&""};
            panic!("Only either #RRGGBB or #RRGGBBAA format is currently acceptable, found '{}'{}", s, quote_warn)
        }
        let r = u16::from_str_radix(&s[1..3], 16).unwrap() as u8;
        let g = u16::from_str_radix(&s[3..5], 16).unwrap() as u8;
        let b = u16::from_str_radix(&s[5..7], 16).unwrap() as u8;

        let a = if s.len() == 9 {
            u16::from_str_radix(&s[7..9], 16).unwrap()
        }
        else {255};

        Self{r,g,b,a: a as u8}
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

    fn set_source(&self, window: &Window, maxheight: f64) {

        let c = &window.ctx;

        let norm = |x| (x as f64) / 255.0;

        match self {
            Self::Color(col) => {c.set_source_rgba(norm(col.r), norm(col.g), norm(col.b), norm(col.a))},
            Self::VGradient(v) => {
                let src = cairo::LinearGradient::new(0.0, 0.0 as f64, 0.0, maxheight as f64);
                for (c,i) in v.iter().enumerate() {
                    src.add_color_stop_rgba(c as f64 / (v.len()-1) as f64, norm(i.r),norm(i.g),norm(i.b), norm(i.a));
                }
                c.set_source(&src).log("set_source");
            }
        }
    }

    pub fn draw_rect(&self, window: &Window, x: f64, y: f64, width: f64, height: f64, maxheight: f64) {

        let c = &window.ctx;

        self.set_source(window, maxheight);
        
        c.set_operator(Operator::Source);
        c.rectangle(x, y, width, height);
        c.fill().log("draw_rect");
    }

    pub fn draw_glyphs(&self, window: &Window, glyphs: &GlyphSet, x_off: f64, y_off: f64, font: &Font, maxheight: f64) {
        
        let c = &window.ctx;

        self.set_source(window, maxheight);
        
        match &glyphs.glyphs {
            GlyphObj::Str(font_id, font_height, g) => {
                
                font.with_scaled_font(*font_id, *font_height, |sfont| {
                    //let text = &ch.to_string();
                    c.set_operator(Operator::Over);
                    
                    c.set_scaled_font(sfont);
                    let (ascent, descent) = c.font_extents()
                        .and_then(|e| Ok((e.ascent, e.descent)))
                        .unwrap_or_else(|_| {eprintln!("Failed to get font ascent"); (0.0, 0.0)});

                    let x = glyphs.x + x_off;
                    let y = glyphs.y + y_off + ascent - descent;
                    let g = g.iter().map(|g| Glyph {index: g.index, x: g.x+x, y: g.y+y}).collect::<Vec<_>>();

                    c.show_glyphs(&g[..]).log("draw_glyphs");
                });
            }
        };
    }
}

impl DrawableSet {

    pub fn from(properties: &WidgetPropertiesCurrent) -> Self {
        Self {
            foreground: properties.foreground.clone(),
            background: properties.background.clone(),
            
            black: properties.black.clone(),
            red: properties.red.clone(),
            green: properties.green.clone(),
            yellow: properties.yellow.clone(),
            blue: properties.blue.clone(),
            magenta: properties.magenta.clone(),
            cyan: properties.cyan.clone(),
            white: properties.white.clone(),
            
            bright_black: properties.bright_black.clone(),
            bright_red: properties.bright_red.clone(),
            bright_green: properties.bright_green.clone(),
            bright_yellow: properties.bright_yellow.clone(),
            bright_blue: properties.bright_blue.clone(),
            bright_magenta: properties.bright_magenta.clone(),
            bright_cyan: properties.bright_cyan.clone(),
            bright_white: properties.bright_white.clone(),

            warn: properties.warn,
            critical: properties.critical,
            dim: properties.dim
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

    /// Append red/yellow/grey colors depending on whether the value is below/above a certain treshold
    pub fn mark_color(&self, value: Option<f64>) -> &str {
        match value {
            None => &"",
            Some(x) => if x >= self.critical {
                &"\x1b[031m"
            }
            else if x >= self.warn {
                &"\x1b[033m"
            }
            else if x <= self.dim {
                &"\x1b[090m"
            }
            else {&""}
        }
    }

    pub fn draw_widget(
        &self,
        window: &Window,
        info: &DrawFGInfo,
        font: &Font,
        offset: i16,
        width_max: u16)
    {
        let lrborder = (width_max - info.width) as f64 / 2.0;
        
        // Background  TODO: varying backgrounds
        info.gsets.first().and_then::<Option<u8>, _>(|s| {s.bg.draw_rect(window, info.x as f64 + offset as f64, 0.0, width_max as f64, info.height as f64, info.height as f64); None});
        for i in &info.gsets {
            // Foreground
            i.fg.draw_glyphs(window, &i, info.x as f64 + offset as f64 + lrborder, info.y as f64, font, info.height as f64);
        }
    }
}

