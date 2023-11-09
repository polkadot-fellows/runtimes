# partial_sort

[![Build Status](https://github.com/sundy-li/partial_sort/actions/workflows/Build.yml/badge.svg)](https://github.com/sundy-li/partial_sort/actions/workflows/Build.yml)
[![](http://meritbadge.herokuapp.com/partial_sort)](https://crates.io/crates/partial_sort)
[![](https://img.shields.io/crates/d/partial_sort.svg)](https://crates.io/crates/partial_sort)
[![](https://img.shields.io/crates/dv/partial_sort.svg)](https://crates.io/crates/partial_sort)
[![](https://docs.rs/partial_sort/badge.svg)](https://docs.rs/partial_sort/)


partial_sort is Rust version of [std::partial_sort](https://en.cppreference.com/w/cpp/algorithm/partial_sort)

## Usage

```rust 
use partial_sort::PartialSort;

fn main() {
    let mut vec = vec![4, 4, 3, 3, 1, 1, 2, 2];
    vec.partial_sort(4, |a, b| a.cmp(b));
    println!("{:?}", vec);
}

```


## Benches
First we compare what happens when sorting the entire vector (in Macbook pro M1Max):

```
partial sort 10000 limit 20                 time:   [5.2093 µs 5.2470 µs 5.2892 µs]
partial sort 10000 limit 200                time:   [15.742 µs 16.116 µs 16.645 µs]
partial sort 10000 limit 2000               time:   [117.99 µs 119.59 µs 121.32 µs]
partial sort 10000 limit 10000              time:   [281.40 µs 287.69 µs 295.43 µs]
stdsort 10000                               time:   [363.75 µs 366.93 µs 371.06 µs]
heapsort 10000                              time:   [253.89 µs 256.02 µs 258.35 µs]

partial reverse sort 10000 limit 20         time:   [5.7620 µs 6.0627 µs 6.5347 µs]
stdsort reverse 10000                       time:   [347.39 µs 355.88 µs 369.46 µs]
```


## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
