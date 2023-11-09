## [Documentation](https://docs.rs/exit-future)
----

Create a `Signal` and cloneable `Exit` future that fires when `Signal` is fired or dropped.
Used to coordinate exit between multiple event-loop threads.

```rust
let (signal, exit) = exit_future::signal();

::std::thread::spawn(move || {
    // future resolves when signal fires
    exit.wait();
});

let _ = signal.fire(); // also would fire on drop.
```
