// Copyright 2021 Olivier Kannengieser
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

///! Macros for static_init crate.
extern crate proc_macro;
extern crate syn;
use syn::spanned::Spanned;
use syn::*;

use core::result::Result;

extern crate quote;
use quote::quote_spanned;

use proc_macro::TokenStream;

extern crate proc_macro2;
use proc_macro2::{Span, TokenStream as TokenStream2};

macro_rules! ok_or_return {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(t) => return t.into(),
        }
    };
}

#[proc_macro_attribute]
pub fn constructor(args: TokenStream, input: TokenStream) -> TokenStream {
    let priority = ok_or_return!(parse_priority(args));

    let section = ok_or_return!(init_section(priority));

    let func: ItemFn = parse_macro_input!(input);

    let func_ptr_name = format!("__static_init_constructor_{}", func.sig.ident);

    let func_type = get_init_func_sig(&func.sig);

    gen_ctor_dtor(func, &section, &func_ptr_name, func_type).into()
}

fn get_init_func_sig(sig: &Signature) -> TypeBareFn {
    let sp = sig.span().resolved_at(Span::mixed_site());

    if cfg!(target_env = "gnu") && cfg!(target_family = "unix") && !sig.inputs.is_empty() {
        parse2(quote_spanned!(sp=>extern "C" fn(i32,*const*const u8, *const *const u8)))
            .unwrap()
    } else {
        parse2(quote_spanned!(sp=>extern "C" fn())).unwrap()
    }
}

fn const_dtor_no_support() -> TokenStream {
    quote_spanned!(Span::mixed_site()=>compile_error!(
        "program constructors/destructors not supported on this target"
    ))
    .into()
}

fn init_section(priority: u16) -> Result<String, TokenStream> {
    //Todo priority bellow(65535-100) should be unsafe
    //or increase the number to be above 100
    //
    //idem put Room for lesser lazy initialized later
    if cfg!(elf) {
        // on linux the standard library args are initilized
        // at .init_array.00099. => priority 65436
        Ok(format!(".init_array.{:05}", 65535 - priority))
    } else if cfg!(mach_o) {
        //on mach it is not clear of ObjC runtime is initialized
        //before or after constructors that are here
        if priority != 0 {
            Err(quote_spanned!(Span::mixed_site()=>compile_error!(
                "Constructor priority other than 0 not supported on this plateform."
            ))
            .into())
        } else {
            Ok("__DATA,__mod_init_func".to_string())
        }
    } else if cfg!(coff) {
        // on windows init maybe be called at .CRT$XCU
        // so lets initialization takes place after
        Ok(format!(".CRT$XCU{:05}", 65535 - priority))
    } else {
        Err(const_dtor_no_support())
    }
}

fn fini_section(priority: u16) -> Result<String, TokenStream> {
    if cfg!(elf) {
        // destructors not used by standard library
        Ok(format!(".fini_array.{:05}", 65535 - priority))
    } else if cfg!(mach_o) {
        if priority != 0 {
            Err(quote_spanned!(Span::mixed_site()=>compile_error!(
                "Constructor priority not supported on this plateform."
            ))
            .into())
        } else {
            Ok("__DATA,__mod_term_func".to_string())
        }
    } else if cfg!(coff) {
        // destructors not used by standard library
        Ok(format!(".CRT$XPTZ{:05}", 65535 - priority))
    } else {
        Err(const_dtor_no_support())
    }
}

#[proc_macro_attribute]
pub fn destructor(args: TokenStream, input: TokenStream) -> TokenStream {
    let priority = ok_or_return!(parse_priority(args));

    let section = ok_or_return!(fini_section(priority));

    let func: ItemFn = parse_macro_input!(input);

    let func_ptr_name = format!("__static_init_destructor_{}", func.sig.ident);

    let sp = func.sig.span();
    let func_type = parse2(quote_spanned!(sp.span()=>extern "C" fn())).unwrap();

    gen_ctor_dtor(func, &section, &func_ptr_name, func_type).into()
}


#[proc_macro_attribute]
pub fn dynamic(args: TokenStream, input: TokenStream) -> TokenStream {
    let item: ItemStatic = parse_macro_input!(input);

    let options = ok_or_return!(parse_dyn_options(parse_macro_input!(args)));

    gen_dyn_init(item, options).into()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InitMode {
    Const,
    Lazy,
    LesserLazy,
    Dynamic(u16),
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum DropMode {
    None,
    Drop,
    Finalize,
    Dynamic(u16),
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Tolerance {
    init_fail:         bool,
    registration_fail: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct DynMode {
    init:      InitMode,
    drop:      DropMode,
    tolerance: Tolerance,
    priming:   bool,
}

fn parse_priority(args: TokenStream) -> std::result::Result<u16, TokenStream2> {
    if !args.is_empty() {
        if let Ok(n) = syn::parse(args.clone()).map_err(|e| e.to_compile_error()) {
            let n: Ident = n;
            if n == "__lazy_init" {
                return Ok(1);
            } else if n == "__lazy_init_finished" {
                return Ok(0);
            }
        }
        let lit: Lit = syn::parse(args).map_err(|e| e.to_compile_error())?;
        parse_priority_literal(&lit)
    } else {
        Ok(0)
    }
}

macro_rules! generate_error{
    ($span:expr => $($args:tt),*) => {
        {
        let __expand = [$(generate_error!(@expand $args)),*];
        quote_spanned!($span.resolved_at(Span::mixed_site()) => ::core::compile_error!(::core::concat!(#(#__expand),*)))
        }
    };
    ($($args:tt),*) => {{
        let __expand = [$(generate_error!(@expand $args)),*];
        quote_spanned!(Span::mixed_site()=>::core::compile_error!(::core::concat!(#(#__expand),*)))
    }
    };
    (@expand $v:literal) => {
        quote_spanned!(Span::mixed_site()=>$v)
    };
    (@expand $v:ident) => {
        {
        quote_spanned!(Span::mixed_site()=>::core::stringify!(#$v))
        }
    };

}

fn parse_priority_literal(lit: &Lit) -> Result<u16, TokenStream2> {
    match lit {
        Lit::Int(n) => n.base10_parse::<u16>().map_err(|e| e.to_compile_error()),
        _ => Err(
            generate_error!(lit.span()=>"Expected a priority in the range [0 ; 65535], found `",lit,"`."),
        ),
    }
}

fn parse_dyn_options(args: AttributeArgs) -> std::result::Result<DynMode, TokenStream2> {
    let mut opt = DynMode {
        init:      InitMode::LesserLazy,
        drop:      DropMode::None,
        tolerance: Tolerance {
            init_fail:         true,
            registration_fail: false,
        },
        priming:   false,
    };

    let mut init_set = false;
    let mut drop_set = false;
    macro_rules! check_no_init{
        ($id: expr) => {
            if init_set {
                let __attr_arg = &$id;
                return Err(generate_error!($id.span()=>"Initialization already specified `",__attr_arg,"`"));
            } else {
                init_set = true;
            }
        }
    }
    macro_rules! check_no_drop{
        ($id: expr) => {
            if drop_set {
                let __attr_arg = &$id;
                return Err(generate_error!($id.span()=>"Drop already specified `",__attr_arg,"`"));
            } else {
                drop_set = true;
            }
        }
    }

    macro_rules! unexpected_arg{
        ($id: expr) => {{
            let __unexpected = &$id;
            Err(generate_error!($id.span()=>
                "Unexpected attribute argument `",
                __unexpected,
                "`. Expected either `init[=<u16>]`, `drop[=<u16>]`, `lazy`, `lesser_lazy`, `drop_only=<u16>`, `prime`, `tolerate_leak` or `try_init_once`."
                ))
        }
        }
    }

    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::Path(id)) => {
                let id = if let Some(id) = id.get_ident() {
                    id
                } else {
                    return unexpected_arg!(id);
                };
                if id == "init" {
                    check_no_init!(id);
                    opt.init = InitMode::Dynamic(0);
                } else if id == "drop" {
                    if !cfg!(constructor_destructor) {
                        return Err(generate_error!(id.span()=>"Static drop mode only supported on unixes and windows"))
                    }
                    check_no_drop!(id);
                    opt.drop = DropMode::Drop;
                } else if id == "finalize" {
                    if !cfg!(constructor_destructor) {
                        return Err(generate_error!(id.span()=>"Static finalization mode only supported on unixes and windows"))
                    }
                    check_no_drop!(id);
                    opt.drop = DropMode::Finalize;
                } else if id == "lazy" {
                    check_no_init!(id);
                    opt.init = InitMode::Lazy;
                } else if id == "lesser_lazy" {
                    check_no_init!(id);
                    opt.init = InitMode::LesserLazy;
                } else if id == "try_init_once" {
                    opt.tolerance.init_fail = false;
                } else if id == "tolerate_leak" {
                    opt.tolerance.registration_fail = true;
                } else if id == "prime" {
                    opt.priming = true;
                } else {
                    return unexpected_arg!(id);
                }
            }
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                let id = if let Some(id) = nv.path.get_ident() {
                    id
                } else {
                    return unexpected_arg!(nv.path);
                };
                if id == "init" {
                    check_no_init!(id);
                    let priority = parse_priority_literal(&nv.lit)?;
                    opt.init = InitMode::Dynamic(priority);
                } else if id == "drop" {
                    check_no_drop!(id);
                    let priority = parse_priority_literal(&nv.lit)?;
                    opt.drop = DropMode::Dynamic(priority);
                } else if id == "drop_only" {
                    check_no_init!(id);
                    check_no_drop!(id);
                    let priority = parse_priority_literal(&nv.lit)?;
                    opt.init = InitMode::Const;
                    opt.drop = DropMode::Dynamic(priority);
                } else {
                    return unexpected_arg!(id);
                }
            }
            NestedMeta::Lit(lit) => {
                check_no_init!(lit);
                let priority = parse_priority_literal(&lit)?;
                opt.init = InitMode::Dynamic(priority);
            }
            _ => {
                return unexpected_arg!(arg);
            }
        }
    }
    if opt.init == InitMode::LesserLazy && !cfg!(constructor_destructor) {
        opt.init = InitMode::Lazy
    }
    if opt.drop == DropMode::None && opt.tolerance.registration_fail {
        return Err(generate_error!(
            "Unusefull `tolerate_leak`: this static is not dropped, it will always leak. Add \
             `drop` or `finalize` attribute argument if the intent is that this static is dropped."
        ));
    }
    if opt.priming && ! (opt.init== InitMode::Lazy || opt.init == InitMode::LesserLazy) {
        return Err(generate_error!(
            "Only lazy statics can be primed"
        ));
    }
    if (opt.init == InitMode::Lazy || opt.init == InitMode::LesserLazy)
        && !(opt.drop == DropMode::None
            || opt.drop == DropMode::Finalize
            || opt.drop == DropMode::Drop)
    {
        Err(generate_error!("Drop mode not supported for lazy statics."))
    } else if let InitMode::Dynamic(p) = opt.init {
        if !opt.tolerance.init_fail
        /*was try_init_once attribute used*/
        {
            Err(generate_error!(
                "Unusefull `try_init_once` attribute: raw statics initialization is attempted \
                 only once."
            ))
        } else if opt.tolerance.registration_fail
        /*was tolerate_leak attribute used*/
        {
            Err(generate_error!(
                "Unusefull `tolerate_leak` attribute: raw statics are registered for drop at \
                 compile time."
            ))
        } else {
            match opt.drop {
                DropMode::Drop => {
                    opt.drop = DropMode::Dynamic(p);
                    Ok(opt)
                }
                DropMode::Finalize => Err(generate_error!(
                    "Drop mode finalize not supported for global dynamic statics."
                )),
                _ => Ok(opt),
            }
        }
    } else {
        Ok(opt)
    }
}

fn gen_ctor_dtor(
    func: ItemFn,
    section: &str,
    func_ptr_name: &str,
    typ: TypeBareFn,
) -> TokenStream2 {
    let func_ptr_name = Ident::new(func_ptr_name, Span::call_site());

    let section = LitStr::new(section, Span::call_site());

    let func_name = &func.sig.ident;

    let sp = func.sig.span().resolved_at(Span::mixed_site());
    //if func.sig.unsafety.is_none() {
    //    quote_spanned! {sp=>compile_error!("Constructors and destructors must be unsafe functions as \
    //    they may access uninitialized memory regions")}
    //} else {
    quote_spanned! {sp=>
        #func
        #[doc(hidden)]
        #[link_section = #section]
        #[used]
        pub static #func_ptr_name: #typ = #func_name;
    }
    //}
}

fn has_thread_local(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        for seg in &attr.path.segments {
            if seg.ident == "thread_local" {
                return true;
            }
        }
    }
    false
}

fn gen_dyn_init(mut stat: ItemStatic, options: DynMode) -> TokenStream2 {
    //TODO: dropped static must be initialized by unsafe code because
    //if initialization panic this will cause UB TBC.
    //
    //TODO: for lazy statics with leak tolerance => only usefull for thread locals.

    let stat_name = &stat.ident;
    let stat_vis = &stat.vis;

    let stat_generator_name = format!("__StaticInitGeneratorFor_{}", stat_name);

    let stat_generator_name = Ident::new(&stat_generator_name, Span::call_site());

    let err = generate_error!(stat.expr.span()=>
        "Expected an expression of the form `match INIT { PRIME => /*expr/*, DYN => /*expr*/}`"
    );


    let stat_typ = &*stat.ty;

    let is_thread_local = has_thread_local(&stat.attrs);

    if is_thread_local && !(options.init == InitMode::Lazy || options.init == InitMode::LesserLazy)
    {
        return generate_error!(
            "Only statics with `#[dynamic(lazy)]` or `#[dynamic(lazy,drop)]` can also have \
             `#[thread_local]` attribute"
        );
    }

    let stat_ref: Expr =
        if !(options.init == InitMode::Lazy || options.init == InitMode::LesserLazy) {
            parse_quote! {
                &mut #stat_name
            }
        } else {
            parse_quote! {
                &#stat_name
            }
        };

    macro_rules! into_mutable {
        () => {
            stat.mutability = Some(token::Mut {
                span: stat.ty.span(),
            })
        };
    }
    macro_rules! into_immutable {
        () => {
            stat.mutability = None
        };
    }

    let typ: Type = if !(options.init == InitMode::Lazy || options.init == InitMode::LesserLazy) {
        if stat.mutability.is_none() {
            into_mutable!();
            parse_quote! {
                ::static_init::raw_static::ConstStatic::<#stat_typ>
            }
        } else {
            parse_quote! {
                ::static_init::raw_static::Static::<#stat_typ>
            }
        }
    } else if is_thread_local && options.priming && options.drop == DropMode::None {
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::UnSyncPrimedLockedLazy::<#stat_typ,#stat_generator_name>
            }
        }
    } else if is_thread_local && options.priming {
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::UnSyncPrimedLockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.priming && options.init == InitMode::Lazy && options.drop == DropMode::None {
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::PrimedLockedLazy::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.priming && options.init == InitMode::LesserLazy && options.drop == DropMode::None {
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::PrimedLesserLockedLazy::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.priming  && options.init == InitMode::Lazy{
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::PrimedLockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.priming  && options.init == InitMode::LesserLazy{
        if stat.mutability.is_none() {
            return generate_error!(stat.static_token.span()=>
                "Primed statics are mutating (safe). Add the `mut` keyword."
            );
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::PrimedLesserLockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if is_thread_local && options.drop == DropMode::Finalize {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::UnSyncLazyFinalize::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::UnSyncLockedLazyFinalize::<#stat_typ,#stat_generator_name>
            }
        }
    } else if is_thread_local && options.drop == DropMode::Drop {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::UnSyncLazyDroped::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::UnSyncLockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if is_thread_local {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::UnSyncLazy::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::UnSyncLockedLazy::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.drop == DropMode::Finalize && options.init == InitMode::LesserLazy {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::LesserLazyFinalize::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::LesserLockedLazyFinalize::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.drop == DropMode::Finalize && options.init == InitMode::Lazy {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::LazyFinalize::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::LockedLazyFinalize::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.drop == DropMode::Drop && options.init == InitMode::Lazy {
        if stat.mutability.is_none() {
            return generate_error!("Droped lazy must be mutable");
            //is_const_droped = true;
            //parse_quote! {
            //    ::static_init::lazy::ConstLockedLazyDroped::<#stat_typ,#stat_generator_name>
            //}
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::LockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.drop == DropMode::Drop && options.init == InitMode::LesserLazy {
        if stat.mutability.is_none() {
            return generate_error!("Droped lazy must be mutable");
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::LesserLockedLazyDroped::<#stat_typ,#stat_generator_name>
            }
        }
    } else if options.init == InitMode::LesserLazy {
        if stat.mutability.is_none() {
            parse_quote! {
                ::static_init::lazy::LesserLazy::<#stat_typ,#stat_generator_name>
            }
        } else {
            into_immutable!();
            parse_quote! {
                ::static_init::lazy::LesserLockedLazy::<#stat_typ,#stat_generator_name>
            }
        }
    } else if stat.mutability.is_none() {
        parse_quote! {
            ::static_init::lazy::Lazy::<#stat_typ,#stat_generator_name>
        }
    } else {
        into_immutable!();
        parse_quote! {
            ::static_init::lazy::LockedLazy::<#stat_typ,#stat_generator_name>
        }
    };

    let (expr, prime_expr) = if !options.priming {
        (&*stat.expr, None)
    } else if let Expr::Match(mexp) = &*stat.expr {
        if let Expr::Path(p) = &*mexp.expr {
            if !p.path.segments.len() == 1 && p.path.segments.first().unwrap().ident == "INIT" {
                return generate_error!(mexp.expr.span()=>
                "Expected `INIT` because the static has `#[dynamic(prime)]` attribute."
                );
            }
        } else {
            return generate_error!(mexp.expr.span()=>
            "Expected `INIT` because the static has `#[dynamic(prime)]` attribute."
            );
        }
        if mexp.arms.len() != 2 {
            return generate_error!(mexp.span()=>
            "Expected two match arms as the static has `#[dynamic(prime)]` attribute."
            );
        }
        let mut expr = None;
        let mut prime_expr = None;
        for arm in &mexp.arms {
            let p = match &arm.pat {
                Pat::Ident(p)
                    if p.by_ref.is_none() && p.mutability.is_none() && p.subpat.is_none() =>
                {
                    p
                }
                x => {
                    return generate_error!(x.span()=>
                    "Expected either `DYN` or `PRIME` as the static has `#[dynamic(prime)]` attribute."
                    )
                }
            };
            if p.ident == "PRIME" && prime_expr.is_none() {
                prime_expr = Some(&*arm.body);
            } else if p.ident == "DYN" && expr.is_none() {
                expr = Some(&*arm.body);
            } else {
                return generate_error!(p.span()=>
                "Repeated match expression `", p, "`. There must be one arm that matches `PRIME` and the other `DYN`."
                );
            }
        }
        (expr.unwrap(), prime_expr)
    } else {
        return err;
    };

    let sp = stat.expr.span().resolved_at(Span::mixed_site());

    let initer = match options.init {
        //InitMode::Dynamic(priority) if options.drop == DropMode::Drop => {
        //    let attr: Attribute = parse_quote_spanned!(Span::mixed_site()=>
        //    #[::static_init::constructor(#priority)]);
        //    Some(quote_spanned! {sp=>
        //            extern "C" fn __static_init_dropper() {
        //                unsafe{#typ::drop(#stat_ref)}
        //            }
        //            #attr
        //            extern "C" fn __static_init_initializer() {
        //                ::static_init::raw_static::__set_init_prio(#priority as i32);
        //                let __static_init_expr_result = #expr;
        //                unsafe {#typ::set_to(#stat_ref,__static_init_expr_result);
        //                ::libc::atexit(__static_init_dropper)};
        //                ::static_init::raw_static::__set_init_prio(i32::MIN);
        //            }
        //    })
        //}
        //InitMode::Dynamic(priority) if options.drop == DropMode::Finalize => {
        //    let attr: Attribute = parse_quote_spanned!(Span::mixed_site()=>#[::static_init::constructor(#priority)]);
        //    Some(quote_spanned! {sp=>
        //            extern "C" fn __static_init_dropper() {
        //                unsafe{::static_init::Finaly::finalize(**#stat_ref)}
        //            }
        //            #attr
        //            extern "C" fn __static_init_initializer() {
        //                ::static_init::raw_static::__set_init_prio(#priority as i32);
        //                let __static_init_expr_result = #expr;
        //                unsafe {#typ::set_to(#stat_ref,__static_init_expr_result);
        //                ::libc::atexit(__static_init_dropper)};
        //                ::static_init::raw_static::__set_init_prio(i32::MIN);
        //            }
        //    })
        //}
        InitMode::Dynamic(priority) => {
            let attr: Attribute = parse_quote!(#[::static_init::constructor(#priority)]);
            Some(quote_spanned! {sp=>
                    #attr
                    extern "C" fn __static_init_initializer() {
                        ::static_init::raw_static::__set_init_prio(#priority as i32);
                        let __static_init_expr_result = #expr;
                        unsafe {#typ::set_to(#stat_ref,__static_init_expr_result)};
                        ::static_init::raw_static::__set_init_prio(i32::MIN);
                    }
            })
        }

        InitMode::LesserLazy if !is_thread_local && cfg!(support_priority) => {
            Some(quote_spanned! {sp=>
                    #[::static_init::constructor(__lazy_init)]
                    extern "C" fn __static_init_initializer() {
                        unsafe {#typ::init(#stat_ref)};
                    }
            })
        }

        InitMode::Const | InitMode::Lazy | InitMode::LesserLazy => None,
    };

    let droper = if let DropMode::Dynamic(priority) = options.drop {
        let attr: Attribute = parse_quote!(#[::static_init::destructor(#priority)]);
        Some(quote_spanned! {sp=>
                #attr
                extern "C" fn __static_init_droper() {
                    unsafe {#typ::drop(#stat_ref)}
                }
        })
    } else {
        None
    };

    let statid = &stat.ident;

    let init_priority: Expr = match options.init {
        InitMode::Dynamic(n) => parse_quote!(::static_init::InitMode::ProgramConstructor(#n)),
        InitMode::Lazy => parse_quote!(::static_init::InitMode::Lazy),
        InitMode::LesserLazy => parse_quote!(::static_init::InitMode::LesserLazy),
        InitMode::Const => parse_quote!(::static_init::InitMode::Const),
    };

    let drop_priority: Expr = match options.drop {
        DropMode::Dynamic(n) => parse_quote!(::static_init::FinalyMode::ProgramDestructor(#n)),
        DropMode::Finalize => parse_quote!(::static_init::FinalyMode::Finalize),
        DropMode::Drop => parse_quote!(::static_init::FinalyMode::Drop),
        DropMode::None => parse_quote!(::static_init::FinalyMode::None),
    };

    let static_info: Option<Expr> = if cfg!(debug_mode) {
        Some(parse_quote!(
        ::static_init::StaticInfo{
            variable_name: ::core::stringify!(#statid),
            file_name: ::core::file!(),
            line: ::core::line!(),
            column: ::core::column!(),
            init_mode: #init_priority,
            drop_mode: #drop_priority
            }))
    } else {
        None
    };

    let init_fail_tol = options.tolerance.init_fail;
    let reg_fail_tol = options.tolerance.registration_fail;

    let lazy_generator = if matches!(options.init, InitMode::Lazy | InitMode::LesserLazy) {
        Some(quote_spanned! {sp=>
            #[allow(clippy::upper_case_acronyms)]
            #stat_vis struct #stat_generator_name;
            impl ::static_init::Generator<#stat_typ> for #stat_generator_name {
                #[inline]
                fn generate(&self) -> #stat_typ {
                    #expr
                }
            }
            impl ::static_init::GeneratorTolerance for #stat_generator_name {
                const INIT_FAILURE: bool = #init_fail_tol;
                const FINAL_REGISTRATION_FAILURE: bool = #reg_fail_tol;
            }
        })
    } else {
        None
    };

    let const_init = match options.init {
        InitMode::Dynamic(_) => {
            quote_spanned! {sp=>{
                #initer
                #droper
                unsafe{#typ::uninit(#static_info)}
            }
            }
        }
        InitMode::Lazy | InitMode::LesserLazy if options.priming && cfg!(debug_mode) => {
            quote_spanned! {sp=> {
                #initer

                let _ = ();

                unsafe{#typ::from_generator_with_info(#prime_expr,#stat_generator_name, #static_info)}
            }
            }
        }
        InitMode::Lazy | InitMode::LesserLazy if options.priming => {
            quote_spanned! {sp=> {
                #initer

                let _ = ();

                unsafe{#typ::from_generator(#prime_expr,#stat_generator_name)}
            }
            }
        }
        InitMode::Lazy | InitMode::LesserLazy if cfg!(debug_mode) => {
            quote_spanned! {sp=> {
                #initer

                let _ = ();

                unsafe{#typ::from_generator_with_info(#stat_generator_name, #static_info)}
            }
            }
        }
        InitMode::Lazy | InitMode::LesserLazy => {
            quote_spanned! {sp=>{
                #initer

                let _ = ();

                unsafe{#typ::from_generator(#stat_generator_name)}
            }
            }
        }
        InitMode::Const => {
            quote_spanned! {sp=>{
                #initer
                #droper
                #typ::from(#expr, #static_info)
            }
            }
        }
    };

    *stat.expr = match parse(const_init.into()) {
        Ok(exp) => exp,
        Err(e) => return e.to_compile_error(),
    };

    *stat.ty = typ;

    quote_spanned! {sp=>
    #lazy_generator
    #stat
    }
}
