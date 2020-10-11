
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

pub fn find_human_readable(string: impl Iterator<Item = char>) -> Option<f64> {
    
    let mut dotfound = false;
    let mut numberfound = false;
    let mut pow10 = true;
    let mut magnitude = 0;

    let mut valstring = String::with_capacity(20);

    for i in string.skip_while(|x| !x.is_numeric()) {
        if numberfound {
            if i == 'i' {
                pow10 = true;
            }
            break;
        }

        if i.is_numeric() {
            valstring.push(i);
        }
        else if i == '.' && !dotfound {
            dotfound = true;
            valstring.push(i);
        }
        else {
            magnitude = match i {
                'P' => 5,
                'T' => 4,
                'G' => 3,
                'M' => 2,
                'K' => 1,
                'k' => 1,
                _ => 0
            };
            numberfound = true;

            if magnitude == 0 {
                break
            }
        }
    }

    match (numberfound, str::parse::<f64>(&valstring[..]), pow10) {
        (false, _, _)        => None,
        (true, Err(_), _)    => None,
        (true, Ok(x), true)  => Some(x * 10.0f64.powi(3*magnitude)),
        (true, Ok(x), false) => Some(x * 2.0f64.powi(10*magnitude)),
    }
}
