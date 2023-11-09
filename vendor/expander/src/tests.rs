use super::*;
use proc_macro2::Span;

#[test]
fn dry() -> Result<(), std::io::Error> {
    let ts = quote! {
        pub struct X {
            x: [u8;32],
        }
    };
    let modified = Expander::new("foo")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        .dry(true)
        .write_to_out_dir(ts.clone())?;

    assert_eq!(
        ts.to_string(),
        modified.to_string(),
        "Dry does not alter the provided `TokenStream`. qed"
    );
    Ok(())
}

#[test]
fn basic() -> Result<(), std::io::Error> {
    let ts = quote! {
        pub struct X {
            x: [u8;32],
        }
    };
    let modified = Expander::new("bar")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .write_to_out_dir(ts.clone())?;

    let s = modified.to_string();
    assert_ne!(s, ts.to_string());
    assert!(s.contains("include ! ("));
    Ok(())
}

#[test]
fn syn_ok_is_written_to_external_file() -> Result<(), std::io::Error> {
    let ts = Ok(quote! {
        pub struct X {
            x: [u8;32],
        }
    });
    let result = Expander::new("bar")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .maybe_write_to_out_dir(ts.clone())?;
    let modified = result.expect("Is not a syn error. qed");

    let s = modified.to_string();
    assert_ne!(s, ts.unwrap().to_string());
    assert!(s.contains("include ! "));
    Ok(())
}

#[test]
fn syn_error_is_not_written_to_external_file() -> Result<(), std::io::Error> {
    const MSG: &str = "Hajajajaiii!";
    let ts = Err(syn::Error::new(Span::call_site(), MSG));
    let result = Expander::new("")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .maybe_write_to_out_dir(ts.clone())?;
    let modified = result.expect_err("Is a syn error. qed");

    let s = modified.to_compile_error().to_string();
    assert!(dbg!(&s).contains("compile_error !"));
    assert!(s.contains(MSG));

    Ok(())
}
