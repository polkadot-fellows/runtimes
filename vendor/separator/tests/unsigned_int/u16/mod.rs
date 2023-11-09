use separator::Separatable;

#[test]
fn nine() {
    let i : u16 = 9;
    assert_eq!("9", &i.separated_string());
}

#[test]
fn ninety() {
    let i : u16 = 90;
    assert_eq!("90", &i.separated_string());
}

#[test]
fn nine_hundred() {
    let i : u16 = 900;
    assert_eq!("900", &i.separated_string());
}

#[test]
fn nine_thousand() {
    let i : u16 = 9000;
    assert_eq!("9,000", &i.separated_string());
}
