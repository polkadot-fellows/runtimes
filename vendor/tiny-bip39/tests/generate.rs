use bip39::{Language, Mnemonic, MnemonicType, Seed};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

fn test_word_count(expected_word_count: usize) {
    let mnemonic_type = MnemonicType::for_word_count(expected_word_count).unwrap();

    let mnemonic = Mnemonic::new(mnemonic_type, Language::English);
    let actual_word_count = mnemonic.phrase().split(" ").count();

    assert_eq!(actual_word_count, expected_word_count);
    assert_eq!(mnemonic_type.word_count(), expected_word_count);

    let seed = Seed::new(&mnemonic, "");
    let seed_bytes: &[u8] = seed.as_bytes();

    assert!(seed_bytes.len() == 64);
}

macro_rules! test_maybe_wasm {
    ($name:ident, $body:expr) => {
        #[cfg_attr(all(target_arch = "wasm32"), wasm_bindgen_test)]
        #[cfg_attr(not(target_arch = "wasm32"), test)]
        fn $name() {
            $body
        }
    }
}

test_maybe_wasm!(generate_12_english, {
    test_word_count(12);
});

test_maybe_wasm!(generate_15_english, {
    test_word_count(15);
});

test_maybe_wasm!(generate_18_english, {
    test_word_count(18);
});

test_maybe_wasm!(generate_21_english, {
    test_word_count(21);
});

test_maybe_wasm!(generate_24_english, {
    test_word_count(24);
});

test_maybe_wasm!(generate_from_invalid_entropy, {
    // 15 bytes
    let entropy = &[
        0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84, 0x6A,
    ];

    assert!(Mnemonic::from_entropy(entropy, Language::English).is_err());
});
