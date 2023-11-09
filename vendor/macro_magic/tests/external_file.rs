use macro_magic::*;

#[export_tokens]
fn external_fn_with_println() {
    println!("testing");
}
