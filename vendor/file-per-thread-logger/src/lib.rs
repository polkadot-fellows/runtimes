#[macro_use]
extern crate log;
extern crate env_logger;

use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use env_logger::filter::{Builder, Filter};
use log::{LevelFilter, Metadata, Record};

thread_local! {
    static WRITER: RefCell<Option<io::BufWriter<File>>> = RefCell::new(None);
}

static ALLOW_UNINITIALIZED: AtomicBool = AtomicBool::new(false);

/// Format function to print logs in a custom format.
pub type FormatFn = fn(&mut io::BufWriter<File>, &Record) -> io::Result<()>;

/// Initializes the current process/thread with a logger, parsing the RUST_LOG environment
/// variables to set the logging level filter and/or directives to set a filter by module name,
/// following the usual env_logger conventions.
///
/// Must be called on every running thread, or else logging will panic the first time it's used.
/// ```
/// use file_per_thread_logger::initialize;
///
/// initialize("log-file-prefix");
/// ```
pub fn initialize(filename_prefix: &str) {
    init_logging(filename_prefix, None)
}

/// Initializes the current process/thread with a logger, parsing the RUST_LOG environment
/// variables to set the logging level filter and/or directives to set a filter by module name,
/// following the usual env_logger conventions. The format function specifies the format in which
/// the logs will be printed.
///
/// Must be called on every running thread, or else logging will panic the first time it's used.
/// ```
/// use file_per_thread_logger::{initialize_with_formatter, FormatFn};
/// use std::io::Write;
///
/// let formatter: FormatFn = |writer, record| {
///     writeln!(
///         writer,
///         "{} [{}:{}] {}",
///         record.level(),
///         record.file().unwrap_or_default(),
///         record.line().unwrap_or_default(),
///         record.args()
///     )
/// };
/// initialize_with_formatter("log-file-prefix", formatter);
/// ```
pub fn initialize_with_formatter(filename_prefix: &str, formatter: FormatFn) {
    init_logging(filename_prefix, Some(formatter))
}

/// Allow logs files to be created from threads in which the logger is specifically uninitialized.
/// It can be useful when you don't have control on threads spawned by a dependency, for instance.
///
/// Should be called before calling code that spawns the new threads.
pub fn allow_uninitialized() {
    ALLOW_UNINITIALIZED.store(true, Ordering::Relaxed);
}

fn init_logging(filename_prefix: &str, formatter: Option<FormatFn>) {
    let env_var = env::var_os("RUST_LOG");
    if env_var.is_none() {
        return;
    }

    let level_filter = {
        let mut builder = Builder::new();
        builder.parse(env_var.unwrap().to_str().unwrap());
        builder.build()
    };

    // Ensure the thread local state is always properly initialized.
    WRITER.with(|rc| {
        if rc.borrow().is_none() {
            rc.replace(Some(open_file(filename_prefix)));
        }
    });

    let logger = FilePerThreadLogger::new(level_filter, formatter);
    let _ =
        log::set_boxed_logger(Box::new(logger)).map(|()| log::set_max_level(LevelFilter::max()));

    info!("Set up logging; filename prefix is {}", filename_prefix);
}

struct FilePerThreadLogger {
    filter: Filter,
    formatter: Option<FormatFn>,
}

impl FilePerThreadLogger {
    pub fn new(filter: Filter, formatter: Option<FormatFn>) -> Self {
        FilePerThreadLogger { filter, formatter }
    }
}

impl log::Log for FilePerThreadLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            WRITER.with(|rc| {
                if rc.borrow().is_none() && ALLOW_UNINITIALIZED.load(Ordering::Relaxed) {
                    rc.replace(Some(open_file("")));
                }
                let mut opt_writer = rc.borrow_mut();
                let writer = opt_writer
                    .as_mut()
                    .expect("call the logger's initialize() function first");
                if let Some(format_fn) = &self.formatter {
                    let _ = format_fn(&mut *writer, record);
                } else {
                    let _ = writeln!(*writer, "{} - {}", record.level(), record.args());
                }
            })
        }
    }

    fn flush(&self) {
        WRITER.with(|rc| {
            let mut opt_writer = rc.borrow_mut();
            let writer = opt_writer
                .as_mut()
                .expect("call the logger's initialize() function first");
            let _ = writer.flush();
        });
    }
}

/// Open the tracing file for the current thread.
fn open_file(filename_prefix: &str) -> io::BufWriter<File> {
    let curthread = thread::current();
    let tmpstr;
    let mut path = filename_prefix.to_owned();
    path.extend(
        match curthread.name() {
            Some(name) => name.chars(),
            // The thread is unnamed, so use the thread ID instead.
            None => {
                tmpstr = format!("{:?}", curthread.id());
                tmpstr.chars()
            }
        }
        .filter(|ch| ch.is_alphanumeric() || *ch == '-' || *ch == '_'),
    );
    let file = File::create(path).expect("Can't open tracing file");
    io::BufWriter::new(file)
}
