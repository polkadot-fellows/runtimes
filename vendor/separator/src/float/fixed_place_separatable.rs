pub trait FixedPlaceSeparatable {
    fn separated_string_with_fixed_place(&self, places: usize) -> String;
}

impl FixedPlaceSeparatable for f32 {
    fn separated_string_with_fixed_place(&self, places: usize) -> String {
        let string = format!("{:.*}", places, self);
        separated_float!(string)
    }
}

impl FixedPlaceSeparatable for f64 {
    fn separated_string_with_fixed_place(&self, places: usize) -> String {
        let string = format!("{:.*}", places, self);
        separated_float!(string)
    }
}
