<div align="center">

# array-bytes
### A Collection of Array/Bytes/Hex Utilities.

[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Checks](https://github.com/hack-ink/array-bytes/actions/workflows/checks.yml/badge.svg?branch=main)](https://github.com/hack-ink/array-bytes/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/docsrs/array-bytes)](https://docs.rs/array-bytes)
[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/hack-ink/array-bytes)](https://github.com/hack-ink/array-bytes/tags)
[![GitHub code lines](https://tokei.rs/b1/github/hack-ink/array-bytes)](https://github.com/hack-ink/array-bytes)
[![GitHub last commit](https://img.shields.io/github/last-commit/hack-ink/array-bytes?color=red&style=plastic)](https://github.com/hack-ink/array-bytes)

</div>

## Abilities
#### `TryFromHex` trait
- Convert hex to num
	- type `AsRef<str> -> isize`
	- type `AsRef<str> -> i8`
	- type `AsRef<str> -> i16`
	- type `AsRef<str> -> i32`
	- type `AsRef<str> -> i64`
	- type `AsRef<str> -> i128`
	- type `AsRef<str> -> usize`
	- type `AsRef<str> -> u8`
	- type `AsRef<str> -> u16`
	- type `AsRef<str> -> u32`
	- type `AsRef<str> -> u64`
	- type `AsRef<str> -> u128`

#### `bytes` prefixed functions
- Convert bytes to hex
  - type `AsRef<[u8]> -> String`

#### `hex` prefixed functions
- Convert `HexBytes` to hex
  - type `&[u8] -> &str`
  - e.g. `b"0x..." -> "0x..."`
- Transform hex from `Array`
  - type `&str -> [u8; N]`
- Convert hex to bytes
  - type  `AsRef<[u8]> -> Vec<u8>`
- Convert hex to `Slice`
  - type `AsRef<[u8]> -> &[u8]`
- Transform hex to `T`
  - type `AsRef<[u8]> -> T`
  - e.g. `"0x..." -> [u8; 20] -> H160`

#### `slice` prefixed functions
- Build fixed length `Array` from `Slice`
  - type `&[T] -> [T; N]`
- Transform `Slice` to `G`
  - type `&[T] -> G`
  - e.g. `&[0_u8, ...] -> [u8; 20] -> H160`

#### `vec` prefixed functions
- Build fixed length `Array` from `Vec`
  - type `Vec<T> -> [T; N]`
- Transform `Vec` to `G`
  - type `Vec<T> -> G`
  - e.g. `vec![0_u8,  ...] -> [u8; 20] -> H160`

#### Serde support (require feature `serde`)
- `#[serde(deserialize_with = "array_bytes::hex_deserialize_n_into")]`
  - type `S -> T`
  - e.g. `"0x..." -> H160`
- `#[serde(deserialize_with = "array_bytes::de_hex2num")]`
  - type `S -> Num`
  - e.g. `"0xA" -> 10_u32`
- `#[serde(deserialize_with = "array_bytes::de_hex2bytes")]`
  - type `S -> Vec<u8>`
  - e.g. `"0x00" -> vec![0_u8]`

## Benchmark results
<div align="right"><sub>Friday, November 26th, 2022</sub></div>

```rs
array_bytes::bytes2hex  time:   [37.241 µs 37.321 µs 37.407 µs]
                        change: [-2.2373% -1.9757% -1.7126%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  4 (4.00%) high mild

hex::encode             time:   [132.17 µs 132.42 µs 132.66 µs]
                        change: [-2.4772% -2.2353% -1.9952%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  4 (4.00%) high mild
  1 (1.00%) high severe

rustc_hex::to_hex       time:   [77.565 µs 77.885 µs 78.236 µs]
                        change: [-1.7109% -1.4392% -1.1561%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  4 (4.00%) high mild
  1 (1.00%) high severe

faster_hex::hex_string  time:   [18.049 µs 18.091 µs 18.140 µs]
                        change: [-2.1506% -1.7957% -1.3953%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 7 outliers among 100 measurements (7.00%)
  5 (5.00%) high mild
  2 (2.00%) high severe

faster_hex::hex_encode_fallback
                        time:   [17.978 µs 18.018 µs 18.064 µs]
                        change: [-2.6657% -2.3283% -1.9846%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe

array_bytes::hex2bytes  time:   [119.27 µs 119.54 µs 119.81 µs]
                        change: [-2.5026% -2.2957% -2.0423%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 14 outliers among 100 measurements (14.00%)
  11 (11.00%) high mild
  3 (3.00%) high severe

array_bytes::hex2bytes_unchecked
                        time:   [82.136 µs 82.324 µs 82.531 µs]
                        change: [-55.176% -53.193% -52.029%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 15 outliers among 100 measurements (15.00%)
  13 (13.00%) high mild
  2 (2.00%) high severe

array_bytes::hex2slice  time:   [112.94 µs 113.32 µs 113.78 µs]
                        change: [-1.6410% -1.1545% -0.6772%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe

array_bytes::hex2slice_unchecked
                        time:   [89.416 µs 89.650 µs 89.956 µs]
                        change: [-22.750% -22.423% -22.099%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 14 outliers among 100 measurements (14.00%)
  8 (8.00%) high mild
  6 (6.00%) high severe

hex::decode             time:   [239.97 µs 240.64 µs 241.33 µs]
                        change: [+0.3733% +0.6910% +1.0245%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

hex::decode_to_slice    time:   [162.75 µs 163.12 µs 163.61 µs]
                        change: [-0.4036% -0.0331% +0.3614%] (p = 0.86 > 0.05)
                        No change in performance detected.
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) high mild
  4 (4.00%) high severe

rustc_hex::from_hex     time:   [166.34 µs 167.65 µs 169.03 µs]
                        change: [-1.5255% -0.5122% +0.5735%] (p = 0.33 > 0.05)
                        No change in performance detected.

faster_hex::hex_decode  time:   [38.419 µs 38.613 µs 38.812 µs]
                        change: [-0.9090% -0.3666% +0.1714%] (p = 0.19 > 0.05)
                        No change in performance detected.
Found 4 outliers among 100 measurements (4.00%)
  4 (4.00%) high mild

faster_hex::hex_decode_unchecked
                        time:   [16.122 µs 16.166 µs 16.212 µs]
                        change: [-0.2496% +0.1886% +0.6435%] (p = 0.41 > 0.05)
                        No change in performance detected.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

faster_hex::hex_decode_fallback
                        time:   [16.001 µs 16.039 µs 16.081 µs]
                        change: [-1.1315% -0.7797% -0.4279%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 8 outliers among 100 measurements (8.00%)
  6 (6.00%) high mild
  2 (2.00%) high severe
```

<div align="right">

#### License
<sup>Licensed under either of <a href="LICENSE-APACHE">Apache-2.0</a> or <a href="LICENSE-GPL3">GPL-3.0</a> at your option.</sup>

</div>
