#![feature(test)]

extern crate bip39;
extern crate test;

use test::Bencher;

use bip39::{Language, Mnemonic, MnemonicType, Seed};

#[bench]
fn validate(b: &mut Bencher) {
    let phrase =
        "silly laptop awake length nature thunder category claim reveal supply attitude drip";

    b.iter(|| {
        let _ = Mnemonic::validate(phrase, Language::English);
    });
}

#[bench]
fn from_entropy(b: &mut Bencher) {
    let phrase =
        "silly laptop awake length nature thunder category claim reveal supply attitude drip";
    let m = Mnemonic::from_phrase(phrase, Language::English).unwrap();
    let entropy = m.entropy();

    b.iter(|| {
        let _ = Mnemonic::from_entropy(entropy, Language::English).unwrap();
    });
}

#[bench]
fn new_mnemonic(b: &mut Bencher) {
    b.iter(|| {
        let _ = Mnemonic::new(MnemonicType::Words12, Language::English);
    });
}

#[bench]
fn new_seed(b: &mut Bencher) {
    let phrase =
        "silly laptop awake length nature thunder category claim reveal supply attitude drip";
    let m = Mnemonic::from_phrase(phrase, Language::English).unwrap();

    b.iter(|| {
        let _ = Seed::new(&m, "");
    });
}
