use separator::{FixedPlaceSeparatable, Separatable};

#[test]
fn nine_million() {
    let f : f32 = 9000000.0;
    assert_eq!("9,000,000", &f.separated_string());
}

#[test]
fn negative_ninety_thousand_point_one() {
    let f : f32 = -90000.1;
    assert_eq!("-90,000.1", &f.separated_string());
}

#[test]
fn nine_thousand_point_one_two() {
    let f : f32 = 9000.12;
    assert_eq!("9,000.12", &f.separated_string());
}

#[test]
fn negative_nine_hundred_point_three_five_eight() {
    let f : f32 = -900.358;
    assert_eq!("-900.358", &f.separated_string());
}

#[test]
fn ninety_point_one_point_two_one_three_four() {
    let f : f32 = 90.2134;
    assert_eq!("90.2134", &f.separated_string());
}


#[test]
fn negative_nine_point_five_ish() {
    let f : f32 = -9.558914;
    assert_eq!("-9.558914", &f.separated_string());
}

#[test]
fn format_to_three_places() {
    let f : f32 = -9057.1234;
    assert_eq!("-9,057.123", &f.separated_string_with_fixed_place(3));
}
