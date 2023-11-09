# waitgroup

A WaitGroup waits for a collection of task to finish. 

## Examples

```rust 
use waitgroup::WaitGroup;
use async_std::task;
async {
	let wg = WaitGroup::new();
	for _ in 0..100 {
		let w = wg.worker();
		task::spawn(async move {
			// do work
			drop(w); // drop w means task finished
		};
	}

	wg.wait().await;
}
```
# License

This project is licensed under Apache License, Version 2.0 ([LICENSE](LICENSE) ).

