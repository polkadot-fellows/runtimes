use separator::Separatable;

#[test]
fn nine() {
    let i : u64 = 9;
    assert_eq!("9", &i.separated_string());
}

#[test]
fn ninety() {
    let i : u64 = 90;
    assert_eq!("90", &i.separated_string());
}

#[test]
fn nine_hundred() {
    let i : u64 = 900;
    assert_eq!("900", &i.separated_string());
}

#[test]
fn nine_thousand() {
    let i : u64 = 9000;
    assert_eq!("9,000", &i.separated_string());
}

#[test]
fn ninety_thousand() {
    let i : u64 = 90000;
    assert_eq!("90,000", &i.separated_string());
}

#[test]
fn nine_hundred_thousand() {
    let i : u64 = 900000;
    assert_eq!("900,000", &i.separated_string());
}

#[test]
fn nine_million() {
    let i : u64 = 9000000;
    assert_eq!("9,000,000", &i.separated_string());
}

#[test]
fn ninety_million() {
    let i : u64 = 90000000;
    assert_eq!("90,000,000", &i.separated_string());
}

#[test]
fn nine_hundred_million() {
    let i : u64 = 900000000;
    assert_eq!("900,000,000", &i.separated_string());
}
