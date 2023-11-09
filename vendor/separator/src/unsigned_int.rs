use crate::Separatable;

impl Separatable for u16 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_uint!(string)
    }
}

impl Separatable for u32 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_uint!(string)
    }
}

impl Separatable for u64 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_uint!(string)
    }
}

impl Separatable for u128 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_uint!(string)
    }
}
