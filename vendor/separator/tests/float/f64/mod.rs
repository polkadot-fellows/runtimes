use separator::{FixedPlaceSeparatable ,Separatable};

#[test]
fn negative_ninety_million_point_two_two_two() {
    let f : f64 = -90000000.222;
    assert_eq!("-90,000,000.222", &f.separated_string());
}

#[test]
fn nine_million() {
    let f : f64 = 9000000.0;
    assert_eq!("9,000,000", &f.separated_string());
}

#[test]
fn negative_ninety_thousand_point_one() {
    let f : f64 = -90000.1;
    assert_eq!("-90,000.1", &f.separated_string());
}

#[test]
fn nine_thousand_point_one_two() {
    let f : f64 = 9000.12;
    assert_eq!("9,000.12", &f.separated_string());
}

#[test]
fn negative_nine_hundred_point_three_five_eight() {
    let f : f64 = -900.358;
    assert_eq!("-900.358", &f.separated_string());
}

#[test]
fn ninety_point_one_point_two_one_three_four() {
    let f : f64 = 90.2134;
    assert_eq!("90.2134", &f.separated_string());
}


#[test]
fn negative_nine_point_five_ish() {
    let f : f64 = -9.558914;
    assert_eq!("-9.558914", &f.separated_string());
}

#[test]
fn format_to_two_places() {
    let f : f64 = -9786057.95702;
    assert_eq!("-9,786,057.96", &f.separated_string_with_fixed_place(2));
}
