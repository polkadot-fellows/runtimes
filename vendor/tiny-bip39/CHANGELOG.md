# Note

This is an archive, for new changes see the [releases](https://github.com/maciejhirsz/tiny-bip39/releases) page on GitHub.

## v0.5.1

No source changes, minor version bump due to the addition of license files, which are
included when packaging a crate.

The license specified in the Cargo manifest is still correct.

### Changes

* Add license files (#12) [c9beb96]

## v0.5.0

Should be source compatible with v0.4.0, only the crate dependencies have changed.

From now on, increasing the required major version of a dependency will always increase the
minor version of the crate.

### Changes

* Updated data-encoding to 0.2 (#10) [d966a0e]

## v0.4.1

Should be source compatible with v0.4.0, only the crate dependencies have changed.

### Changes

* Update ring, bitreader, and error-chain [ae9bdfa]

## v0.4.0

Mostly source compatible with v0.3.0, except for the additional error kind, and guarding
against invalid entropy lengths being used to create a Mnemonic.

### Changes

* Add failure test for Mnemonic::from_entropy()  [4be7216]
* Return error when invalid entropy is used  [becd7b1]
* Add error type for invalid entropy length  [cd28a0b]
* Implement std::fmt::Display for MnemonicType  [0b7edf2]

## v0.3.0

Completely refactored internal design and public API for ergonomics as well as to ensure
safe usage. A number of std traits have been implemented for crate types as well.

### Public API

There is now a Mnemonic type that is the "main" type in the crate, it can only be created by
generating a new one, or supplying valid (proper length) entropy to re-create an existing
mnemonic (see warning below).

Once you have a Mnemonic, you can get a Seed from it for deriving HD wallet addresses, but
for safety it cannot be created directly. You must obtain the Seed from a valid Mnemonic
instance.

From crate docs:

    Because it is not possible to create a Mnemonic instance that is invalid, it is
    therefore impossible to have a Seed instance that is invalid. This guarantees
    that only a valid, intact mnemonic phrase can be used to derive HD wallet addresses.

### Warning

If you supply your own entropy to create a Mnemonic rather than generating a new one or
using Mnemonic::from_string(), the Mnemonic instance you get back is by definition **valid**
as far as the BIP39 standard is concerned, even if it doesn't correspond to the phrase you
thought it would.

The BIP39 checksum only covers the actual phrase as a string, if you somehow corrupt the
entropy when storing or transmitting it outside this crate, you will get a different *but still valid*
phrase the next time you use it.

You should *think very carefully* before storing or using entropy values directly rather than the
mnemonic string, they are generally not useful except in advanced use cases and cannot be
used for HD wallet addresses (that's what the Seed is for, which is not the same thing).

### Changes

* Better documentation
    * Add quick start example to docs in crate root [4e3b097]
* Better public API
    * Rename Bip39 struct to Mnemonic [89de089]
    * Add a Seed type [22d9a68]
    * Add Mnemonic::from_entropy and Mnemonic::from_entropy_hex (#9) [cbec489]
    * Add Mnemonic::to_entropy & Mnemonic::to_entropy_hex (#7) [63dec97]
    * Allow Mnemonic to be used as a borrowed or owned string [c6d0162]
    * Implement AsRef<str> for Mnemonic, gets the phrase as a string [e99b4bd]
    * Implement Default for Language and MnemonicType [01d2f46]
    * Derive Clone on Mnemonic and Seed [bdf9933]
    * Derive Copy and Clone for KeyType [9544ded]
    * Derive Copy and Clone for Language [13e3ddb]
    * Use consistent rules for KeyType and Language params, don't require refs [3992510]
* Better error handling
    * Use error-chain (#6) [e878c67]

## v0.2.1

### Changes

* Update rand crate
* Update Ring crate
* Update crypto layer for Ring 0.11

## v0.2.0

Should be source compatible with older versions, none of the public interfaces have changed,
only internal organization and crate dependencies.

### Changes

* Removes rustc-serialize dependency in favor of [data-encoding](https://crates.io/crates/data-encoding)
    * to_hex() is now directly part of the Bip39 struct rather than the ToHex trait
* Replaces rust-crypto with ring
* Removes binary from crate

## 0.1.1

Minor changes to public API, but also removes a panic!() call

### Changes

* Implement std::error::Error and std:fmt::Display on Bip39Error
* Use Into<String> for public function arguments
* Don’t panic if words aren’t found in word list during validation

## 0.1.0

Initial release
