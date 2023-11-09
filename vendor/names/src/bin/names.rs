use names::Generator;

fn main() {
    let args = cli::parse();

    Generator::with_naming(args.naming())
        .take(args.amount)
        .for_each(|name| println!("{}", name));
}

mod cli {
    use clap::Parser;
    use names::Name;

    const AUTHOR: &str = concat!(env!("CARGO_PKG_AUTHORS"), "\n\n");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    pub(crate) fn parse() -> Args {
        Args::parse()
    }

    /// A random name generator with results like "delirious-pail"
    #[derive(Parser, Debug)]
    #[clap(author = AUTHOR, version = VERSION)]
    pub(crate) struct Args {
        /// Adds a random number to the name(s)
        #[clap(short, long)]
        pub(crate) number: bool,

        /// Number of names to generate
        #[clap(default_value = "1", rename_all = "screaming_snake_case")]
        pub(crate) amount: usize,
    }

    impl Args {
        pub(crate) fn naming(&self) -> Name {
            if self.number {
                Name::Numbered
            } else {
                Name::default()
            }
        }
    }
}
