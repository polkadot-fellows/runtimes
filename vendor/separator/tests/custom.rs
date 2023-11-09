#[macro_use] extern crate separator;
use separator::Separatable;

struct CustomNum(u32);

impl Separatable for CustomNum {
    fn separated_string(&self) -> String {
        let string = format!("{}", self.0);
        separated_uint!(string)
    }
}

#[test]
fn nine_hundred_million() {
    let i = CustomNum(900000000);
    assert_eq!("900,000,000", &i.separated_string());
}
