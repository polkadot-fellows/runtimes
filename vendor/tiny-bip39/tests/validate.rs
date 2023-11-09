use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

fn validate_language(lang: Language) {
    let types = &[
        MnemonicType::Words12,
        MnemonicType::Words15,
        MnemonicType::Words18,
        MnemonicType::Words21,
        MnemonicType::Words24,
    ];

    for mtype in types {
        for _ in 0..1000 {
            let m1 = Mnemonic::new(*mtype, lang);
            let m2 = Mnemonic::from_phrase(m1.phrase(), lang).expect("Can create a Mnemonic");

            assert_eq!(m1.entropy(), m2.entropy());
        }
    }
}

macro_rules! test_maybe_wasm {
    ($name:ident, $(#[$attr:meta])+, $body:expr) => {
        #[cfg_attr(all(target_arch = "wasm32"), wasm_bindgen_test)]
        #[cfg_attr(not(target_arch = "wasm32"), test)]
        $(#[$attr])*
        fn $name() {
            $body
        }
    };
    ($name:ident, $body:expr) => {
        #[cfg_attr(all(target_arch = "wasm32"), wasm_bindgen_test)]
        #[cfg_attr(not(target_arch = "wasm32"), test)]
        fn $name() {
            $body
        }
    };
}

test_maybe_wasm!(validate_12_english, {
    let phrase = "park remain person kitchen mule spell knee armed position rail grid ankle";

    let _ = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
});

test_maybe_wasm!(validate_12_english_extra_spaces, {
    let phrase = " park remain  person kitchen mule spell knee armed position rail grid ankle ";
    let clean_phrase = "park remain person kitchen mule spell knee armed position rail grid ankle";

    let mnemonic = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
    let clean_mnemonic =
        Mnemonic::from_phrase(clean_phrase, Language::English).expect("Can create a Mnemonic");

    assert_eq!(mnemonic.entropy(), clean_mnemonic.entropy());
});

test_maybe_wasm!(validate_15_english, {
    let phrase = "any paddle cabbage armor atom satoshi fiction night wisdom nasty they midnight chicken play phone";

    let _ = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
});

test_maybe_wasm!(validate_18_english, {
    let phrase = "soda oak spy claim best oppose gun ghost school use sign shock sign pipe vote follow category filter";

    let _ = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
});

test_maybe_wasm!(validate_21_english, {
    let phrase = "quality useless orient offer pole host amazing title only clog sight wild anxiety gloom market rescue fan language entry fan oyster";

    let _ = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
});

test_maybe_wasm!(validate_24_english, {
    let phrase = "always guess retreat devote warm poem giraffe thought prize ready maple daughter girl feel clay silent lemon bracket abstract basket toe tiny sword world";

    let _ = Mnemonic::from_phrase(phrase, Language::English).expect("Can create a Mnemonic");
});

test_maybe_wasm!(validate_12_english_uppercase, {
    let invalid_phrase =
        "Park remain person kitchen mule spell knee armed position rail grid ankle";

    assert!(Mnemonic::from_phrase(invalid_phrase, Language::English).is_err());
});

test_maybe_wasm!(validate_english, {
    validate_language(Language::English);
});

test_maybe_wasm!(validate_chinese_simplified, #[cfg(feature = "chinese-simplified")], {
    validate_language(Language::ChineseSimplified);
});

test_maybe_wasm!(validate_chinese_traditional, #[cfg(feature = "chinese-traditional")], {
    validate_language(Language::ChineseTraditional);
});

test_maybe_wasm!(validate_french, #[cfg(feature = "french")], {
    validate_language(Language::French);
});

test_maybe_wasm!(validate_italian, #[cfg(feature = "italian")], {
    validate_language(Language::Italian);
});

test_maybe_wasm!(validate_japanese, #[cfg(feature = "japanese")], {
    validate_language(Language::Japanese);
});

test_maybe_wasm!(validate_korean, #[cfg(feature = "korean")], {
    validate_language(Language::Korean);
});

test_maybe_wasm!(validate_spanish, #[cfg(feature = "spanish")], {
    validate_language(Language::Spanish);
});
