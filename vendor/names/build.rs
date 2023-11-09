use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    update_version_if_nightly();

    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);
    let src_dir = Path::new("data");

    generate(
        src_dir.join("adjectives.txt"),
        out_dir.join("adjectives.rs"),
    )?;
    generate(src_dir.join("nouns.txt"), out_dir.join("nouns.rs"))?;
    Ok(())
}

fn generate(src_path: impl AsRef<Path>, dst_path: impl AsRef<Path>) -> io::Result<()> {
    let src = BufReader::new(File::open(src_path.as_ref())?);
    let mut dst = BufWriter::new(File::create(dst_path.as_ref())?);

    writeln!(dst, "[")?;
    for word in src.lines() {
        writeln!(dst, "\"{}\",", &word.unwrap())?;
    }
    writeln!(dst, "]")
}

fn update_version_if_nightly() {
    println!("cargo:rerun-if-env-changed=NIGHTLY_BUILD");
    if let Ok(date) = std::env::var("NIGHTLY_BUILD") {
        println!(
            "cargo:rustc-env=CARGO_PKG_VERSION={}-nightly.{}",
            std::env::var("CARGO_PKG_VERSION")
                .unwrap()
                .split('-')
                .next()
                .unwrap(),
            date,
        );
    }
}
