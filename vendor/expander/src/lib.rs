use fs_err as fs;
use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::io::Write;
use std::path::Path;

/// Rust edition to format for.
#[derive(Debug, Clone, Copy)]
pub enum Edition {
    Unspecified,
    _2015,
    _2018,
    _2021,
}

impl std::default::Default for Edition {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::_2015 => "2015",
            Self::_2018 => "2018",
            Self::_2021 => "2021",
            Self::Unspecified => "",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
enum RustFmt {
    Yes(Edition),
    No,
}

impl std::default::Default for RustFmt {
    fn default() -> Self {
        RustFmt::No
    }
}

impl From<Edition> for RustFmt {
    fn from(edition: Edition) -> Self {
        RustFmt::Yes(edition)
    }
}

/// Expander to replace a tokenstream by a include to a file
#[derive(Default, Debug)]
pub struct Expander {
    /// Determines if the whole file `include!` should be done (`false`) or not (`true`).
    dry: bool,
    /// If `true`, print the generated destination file to terminal.
    verbose: bool,
    /// Filename for the generated indirection file to be used.
    filename: String,
    /// Additional comment to be added.
    comment: Option<String>,
    /// Format using `rustfmt` in your path.
    rustfmt: RustFmt,
}

impl Expander {
    /// Create a new expander.
    pub fn new(filename: impl AsRef<str>) -> Self {
        Self {
            dry: false,
            verbose: false,
            filename: filename.as_ref().to_owned(),
            comment: None,
            rustfmt: RustFmt::No,
        }
    }

    /// Add a header comment.
    pub fn add_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.comment = comment.into().map(|comment| format!("/* {} */\n", comment));
        self
    }

    /// Format the resulting file, for readability.
    pub fn fmt(mut self, edition: impl Into<Edition>) -> Self {
        self.rustfmt = RustFmt::Yes(edition.into());
        self
    }

    /// Do not modify the provided tokenstream.
    pub fn dry(mut self, dry: bool) -> Self {
        self.dry = dry;
        self
    }

    /// Print the path of the generated file to `stderr` during the proc-macro invocation.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    #[cfg(any(feature = "syndicate", test))]
    /// Create a file with `filename` under `env!("OUT_DIR")` if it's not an `Err(_)`.
    pub fn maybe_write_to_out_dir(
        self,
        tokens: impl Into<Result<TokenStream, syn::Error>>,
    ) -> Result<syn::Result<TokenStream>, std::io::Error> {
        self.maybe_write_to(tokens, std::path::PathBuf::from(env!("OUT_DIR")).as_path())
    }

    /// Create a file with `filename` under `env!("OUT_DIR")`.
    pub fn write_to_out_dir(self, tokens: TokenStream) -> Result<TokenStream, std::io::Error> {
        let out = std::path::PathBuf::from(env!("OUT_DIR"));
        self.write_to(tokens, out.as_path())
    }

    #[cfg(any(feature = "syndicate", test))]
    /// Create a file with `filename` at `dest` if it's not an `Err(_)`.
    pub fn maybe_write_to(
        self,
        maybe_tokens: impl Into<Result<TokenStream, syn::Error>>,
        dest_dir: &Path,
    ) -> Result<syn::Result<TokenStream>, std::io::Error> {
        match maybe_tokens.into() {
            Ok(tokens) => Ok(Ok(self.write_to(tokens, dest_dir)?)),
            err => Ok(err),
        }
    }

    /// Create a file with `self.filename` in  `dest_dir`.
    pub fn write_to(
        self,
        tokens: TokenStream,
        dest_dir: &Path,
    ) -> Result<TokenStream, std::io::Error> {
        if self.dry {
            Ok(tokens)
        } else {
            expand_to_file(
                tokens,
                dest_dir.join(self.filename).as_path(),
                dest_dir,
                self.rustfmt,
                self.comment,
                self.verbose,
            )
        }
    }
}

/// Take the leading 6 bytes and convert them to 12 hex ascii characters.
fn make_suffix(digest: &[u8; 32]) -> String {
    let mut shortened_hex = String::with_capacity(12);
    const TABLE: &[u8] = b"0123456789abcdef";
    for &byte in digest.iter().take(6) {
        shortened_hex.push(TABLE[((byte >> 4) & 0x0F) as usize] as char);
        shortened_hex.push(TABLE[((byte >> 0) & 0x0F) as usize] as char);
    }
    shortened_hex
}

/// Expand a proc-macro to file.
///
/// The current working directory `cwd` is only used for the `rustfmt` invocation
/// and hence influences where the config files would be pulled in from.
fn expand_to_file(
    tokens: TokenStream,
    dest: &Path,
    cwd: &Path,
    rustfmt: RustFmt,
    comment: impl Into<Option<String>>,
    verbose: bool,
) -> Result<TokenStream, std::io::Error> {
    let token_str = tokens.to_string();
    let mut bytes = token_str.as_bytes();
    let hash = <blake2::Blake2s256 as blake2::Digest>::digest(bytes);
    let shortened_hex = make_suffix(hash.as_ref());

    let dest =
        std::path::PathBuf::from(dest.display().to_string() + "-" + shortened_hex.as_str() + ".rs");

    if verbose {
        eprintln!("expander: writing {}", dest.display());
    }
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest.as_path())?;

    if let Some(comment) = comment.into() {
        f.write_all(&mut comment.as_bytes())?;
    }

    f.write_all(&mut bytes)?;

    if let RustFmt::Yes(edition) = rustfmt {
        std::process::Command::new("rustfmt")
            .arg(format!("--edition={}", edition))
            .arg(&dest)
            .current_dir(cwd)
            .spawn()?;
    }

    let dest = dest.display().to_string();
    Ok(quote! {
        include!( #dest );
    })
}

#[cfg(test)]
mod tests;
