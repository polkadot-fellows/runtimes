# File per thread logger

This is a thread-safe logger that will write logs to files, each thread owning
its own file.

### Usage

Put this in your `Cargo.toml`:

```
[dependencies]
file-per-thread-logger = "0.1.2"
```

Then add this to your code, once per thread:

```rust
file_per_thread_logger::initialize("file_prefix-");
```

Then each use of `log`'s primitive will log into files named the following way:
- the main thread get a file that's suffixed after the program's name.
- unnamed threads get a file suffixed with ThreadIdN where N is the thread's id
  number.
- named threads get a file suffixed with the thread's name.
