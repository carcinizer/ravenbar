
pub fn mul_comp(a: u8, b: u8) -> u8 {
    ( a as u16 * b as u16 / 256 ) as u8
}

pub fn mix_comp(a: u8, b: u8, factor: f32) -> u8 {
    (b as f32 * factor + a as f32 * (1.0 - factor)) as _
}

pub fn human_readable(n: u64) -> String {
    let (div, suffix) : (u64, &str) = 
        if      n > (1 << 50) {(1 << 50, "Pi")}
        else if n > (1 << 40) {(1 << 40, "Ti")}
        else if n > (1 << 30) {(1 << 30, "Gi")}
        else if n > (1 << 20) {(1 << 20, "Mi")}
        else if n > (1 << 10) {(1 << 10, "Ki")}
        else {(1, "")};

    format!("{:.2}{}", n as f64 / div as f64, suffix)
}

pub fn human_readable_p10(n: u64) -> String {
    let (div, suffix) : (u64, &str) = 
        if      n > (10u64.pow(15)) {(10u64.pow(15), "P")}
        else if n > (10u64.pow(12)) {(10u64.pow(12), "T")}
        else if n > (10u64.pow( 9)) {(10u64.pow( 9), "G")}
        else if n > (10u64.pow( 6)) {(10u64.pow( 6), "M")}
        else if n > (10u64.pow( 3)) {(10u64.pow( 3), "K")}
        else {(1, "")};

    format!("{:.2}{}", n as f64 / div as f64, suffix)
}

