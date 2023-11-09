use separator::Separatable;

#[test]
fn negative_nine_hundred_million() {
    let i : i128 = -900000000;
    assert_eq!("-900,000,000", &i.separated_string());
}

#[test]
fn negative_ninety_million() {
    let i : i128 = -90000000;
    assert_eq!("-90,000,000", &i.separated_string());
}

#[test]
fn negative_nine_million() {
    let i : i128 = -9000000;
    assert_eq!("-9,000,000", &i.separated_string());
}

#[test]
fn negative_nine_hundred_thousand() {
    let i : i128 = -900000;
    assert_eq!("-900,000", &i.separated_string());
}

#[test]
fn negative_ninety_thousand() {
    let i : i128 = -90000;
    assert_eq!("-90,000", &i.separated_string());
}

#[test]
fn negative_nine_thousand() {
    let i : i128 = -9000;
    assert_eq!("-9,000", &i.separated_string());
}

#[test]
fn negative_nine_hundred() {
    let i : i128 = -900;
    assert_eq!("-900", &i.separated_string());
}

#[test]
fn negative_ninety() {
    let i : i128 = -90;
    assert_eq!("-90", &i.separated_string());
}

#[test]
fn negative_nine() {
    let i : i128 = -9;
    assert_eq!("-9", &i.separated_string());
}

#[test]
fn nine() {
    let i : i128 = 9;
    assert_eq!("9", &i.separated_string());
}

#[test]
fn ninety() {
    let i : i128 = 90;
    assert_eq!("90", &i.separated_string());
}

#[test]
fn nine_hundred() {
    let i : i128 = 900;
    assert_eq!("900", &i.separated_string());
}

#[test]
fn nine_thousand() {
    let i : i128 = 9000;
    assert_eq!("9,000", &i.separated_string());
}

#[test]
fn ninety_thousand() {
    let i : i128 = 90000;
    assert_eq!("90,000", &i.separated_string());
}

#[test]
fn nine_hundred_thousand() {
    let i : i128 = 900000;
    assert_eq!("900,000", &i.separated_string());
}

#[test]
fn nine_million() {
    let i : i128 = 9000000;
    assert_eq!("9,000,000", &i.separated_string());
}

#[test]
fn ninety_million() {
    let i : i128 = 90000000;
    assert_eq!("90,000,000", &i.separated_string());
}

#[test]
fn nine_hundred_million() {
    let i : i128 = 900000000;
    assert_eq!("900,000,000", &i.separated_string());
}
