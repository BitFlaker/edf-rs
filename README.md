# Rust EDF 

[![Crates.io Version](https://img.shields.io/crates/v/edf-rs)](https://crates.io/crates/edf-rs)
[![docs.rs](https://img.shields.io/docsrs/edf-rs)](https://docs.rs/edf-rs/latest/edf_rs/)
![GitHub License](https://img.shields.io/github/license/BitFlaker/edf-rs)
![GitHub repo size](https://img.shields.io/github/repo-size/BitFlaker/edf-rs)
![Version state](https://img.shields.io/badge/version_state-nightly-A04EED)

A pure Rust library for reading and writing EDF/EDF+ as well as BDF/BDF+ files. It aims at providing a feature-rich and reliable way to work with `*.edf` and `*.bdf` files. This crate is optimized to work efficiently with very large file sizes. It is based on the official [EDF specification](https://www.edfplus.info/) and [BDF specification](https://www.biosemi.com/faq/file_format.htm). Currently the primary focus for this project is to power my other project [NoctiG Scorer](https://github.com/BitFlaker/noctig-scorer).

**This library is an unofficial implementation. It is still in an early development stage which is not yet considered stable and will have breaking changes in the future.**

## Features and Roadmap
Here is a non-exhaustive list of all implemented and planned features for this library. If you have any ideas or suggestions, feel free to open an [issue](https://github.com/BitFlaker/edf-rs/issues) or a [pull-request](https://github.com/BitFlaker/edf-rs/pulls). Planned features are just a guideline and are not guaranteed to be added in the future.

| Status            | Feature                                                      |
|-------------------|--------------------------------------------------------------|
|  Fully supported  | Reading EDF/EDF+ files                                       |
|  Fully supported  | Reading BDF/BDF+ files                                       |
|  Fully supported  | Creating / Updating existing EDF/EDF+ and BDF/BDF+ files     |
|  Fully supported  | Adding / Removing / Updating existing records and signals    |
|  Fully supported  | Support for seeking                                          |
|  Fully supported  | Reading data by custom duration (nanoseconds, seconds, etc.) |
|  WIP              | Extensive documentation                                      |
|  WIP              | Examples                                                     |
|                   | Support for EDF extensions                                   |
|                   | Conversion from/to other formats (e.g. [OpenBCI Recordings](https://docs.openbci.com/Software/OpenBCISoftware/GUIDocs/#exported-data)) |
|                   | Language bindings for other programming languages            |
|                   | Additional features (e.g. merging files, etc.)               |

<sup>WIP ... Work in Progress</sup>

## Installation
To get started using this crate, first add the dependency to your project using one of the two options listed below.
After that you can refer to the [Usage](#usage) section on how to use the crate.
* Add it to your rust project with the following command:
  ```bash
  cargo add edf-rs
  ```
* or manually add the dependency to your `Cargo.toml` file:
    ```toml
    [dependencies]
    edf-rs = "0.2.1"
    ```

## Usage
The code snippet below shows how to open an EDF+ file, print the metadata and read the first few data-records. For more examples and a full documentation of all functions, see the crate docs at [docs.rs](https://docs.rs/edf-rs/latest/edf_rs/)

```rust
use edf_rs::file::EDFFile;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the EDF+ file from any path
    let mut edf = EDFFile::open("recording.edf")?;

    // Print the debug output for the parsed EDF+ file header
    println!("{:#?}", edf.header);

    // Read all data-records and print the stored annotations
    let record_count = edf.header.get_record_count().unwrap_or(0);
    for i in 0..record_count {
        let Some(record) = edf.read_record()? else {
            continue;
        };

        // Print the annotations (including the time keeping annotation).
        // For regular EDF files, these annotations will always be empty
        // as they are only included in the EDF+ specification
        println!("Annotations in data-record {}", i);
        println!("{:#?}", record.annotations);

        // Do something with the signals. The order of the signals
        // is the same as the signals in the header
        let max_signal_values: Vec<i16> = record.signal_samples.into_iter()
            .map(|samples| samples.into_iter().max().unwrap_or(0))
            .collect();
        println!("Max values: {:?}", max_signal_values)
    }

    Ok(())
}
```

## Disclaimer
I (the developer of this tool) am not a scientist, doctor or similar. I am just a programmer who maintains this tool as a hobby because it is the application I wish existed. This means it is possible that some features of this tool do not work as they should (due to lack of scientific knowledge or similar). This tool is not intended for medical treatment or diagnosis. This software is offered "as is" and it could contain errors, bugs or vulnerabilities which could lead to unexpected or undesirable consequences. If you encounter such problems, feel free to report them in the [issues](https://github.com/BitFlaker/edf-rs/issues) section. Keep in mind that this application is still in a very early development stage and not yet considered stable. I cannot and do not accept any liability for damages related to the use of this software. Use it at your own risk.

## Like this project?
If you like this project, consider giving this repository a star \
It really helps me to stay motivated working on this project when I see it helping others.

## Contributing
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as below, without any additional terms or conditions.

## License
This project is licensed under either of

* Apache License, Version 2.0 [[LICENSE-APACHE](LICENSE-APACHE)]
* MIT License [[LICENSE-MIT](LICENSE-MIT)]

at your option.
