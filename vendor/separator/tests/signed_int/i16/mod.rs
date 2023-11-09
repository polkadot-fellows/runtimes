use separator::Separatable;

#[test]
fn negative_nine_thousand() {
    let i : i16 = -9000;
    assert_eq!("-9,000", &i.separated_string());
}

#[test]
fn negative_nine_hundred() {
    let i : i16 = -900;
    assert_eq!("-900", &i.separated_string());
}

#[test]
fn negative_ninety() {
    let i : i16 = -90;
    assert_eq!("-90", &i.separated_string());
}

#[test]
fn negative_nine() {
    let i : i16 = -9;
    assert_eq!("-9", &i.separated_string());
}

#[test]
fn nine() {
    let i : i16 = 9;
    assert_eq!("9", &i.separated_string());
}

#[test]
fn ninety() {
    let i : i16 = 90;
    assert_eq!("90", &i.separated_string());
}

#[test]
fn nine_hundred() {
    let i : i16 = 900;
    assert_eq!("900", &i.separated_string());
}

#[test]
fn nine_thousand() {
    let i : i16 = 9000;
    assert_eq!("9,000", &i.separated_string());
}
