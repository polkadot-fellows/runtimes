/*
Copyright ⓒ 2017 contributors.
Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
#[macro_use] mod util;

macro_rules! test_pkg {
    ($name:expr) => {
        cargo!("build", "--manifest-path", concat!("tests/pkgs/", $name, "/Cargo.toml"))
            .expect(concat!("failed to build command to build tests/pkgs/", $name))
            .succeeded(concat!("failed to build tests/pkgs/", $name))
    };
}

#[test]
fn test_pkgs() {
    test_pkg!("basic");
    test_pkg!("cargo-env");

    if cfg!(feature = "nightly") {
        test_pkg!("basic-nightly");
    }
}
