use crate::Separatable;

impl Separatable for i16 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_int!(string)
    }
}

impl Separatable for i32 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_int!(string)
    }
}

impl Separatable for i64 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_int!(string)
    }
}

impl Separatable for i128 {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_int!(string)
    }
}
