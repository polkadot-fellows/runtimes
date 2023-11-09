use crate::Separatable;

macro_rules! separated_float {
    ($string:expr) => {{
        let idx = match $string.find('.') {
            Some(i) => i,
            None => $string.len()
        };

        let int_part = &$string[..idx];
        let fract_part = &$string[idx..];

        let mut output = String::new();
        let magnitude = if int_part.starts_with('-') {
            output.push('-');
            int_part[1..].to_owned()
        } else {
            int_part.to_owned()
        };

        let mut place = magnitude.len();
        let mut later_loop = false;

        for ch in magnitude.chars() {
            if later_loop && place % 3 == 0 {
                output.push(',');
            }

            output.push(ch);
            later_loop = true;
            place -= 1;
        };

        output.push_str(fract_part);
        output
    }};
}

impl Separatable for f32 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_float!(string)
    }
}

impl Separatable for f64 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_float!(string)
    }
}
