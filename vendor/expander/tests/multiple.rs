#[baz::baz]
struct A;

#[baz::baz]
struct B;

#[test]
fn main() {
    let _a = A;
    let _b = B;
}
