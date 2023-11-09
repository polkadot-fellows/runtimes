use Separatable;

impl Separatable for usize {
    fn separated_string(&self) -> String {
        let string = format!("{}", self);
        separated_int!(string)
    }
}
