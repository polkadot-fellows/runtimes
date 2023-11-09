use tt_call::{parse_type, tt_call};

tt_call! {
    macro = [{ parse_type }]
    input = [{ <T as :: @ }]
}

fn main() {}
