#[macro_export]
macro_rules! separated_uint_with_output {
    ($string:expr, $output:expr) => {{
        let mut place = $string.len();
        let mut later_loop = false;

        for ch in $string.chars() {
            if later_loop && place % 3 == 0 {
                $output.push(',');
            }

            $output.push(ch);
            later_loop = true;
            place -= 1;
        };

        $output
    }};
}

#[macro_export]
macro_rules! separated_uint {
    ($string:expr) => {{
        let mut output = String::new();
        separated_uint_with_output!($string, output)
    }}
}

#[macro_export]
macro_rules! separated_int {
    ($string:expr) => {{
        let mut output = String::new();
        let magnitude = if $string.starts_with('-') {
            output.push('-');
            (&$string)[1..].to_owned()
        } else {
            $string
        };

        separated_uint_with_output!(magnitude, output)
    }};
}

#[macro_export]
macro_rules! separated_float {
    ($string:expr) => {{
        let idx = match $string.find('.') {
            Some(i) => i,
            None => $string.len()
        };

        let int_part = (&$string[..idx]).to_owned();
        let fract_part = &$string[idx..];
        let output = separated_int!(int_part);
        output + fract_part
    }};
}
