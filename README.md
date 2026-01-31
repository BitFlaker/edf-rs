# Rust EDF 

[![Crates.io Version](https://img.shields.io/crates/v/edf-rs)](https://crates.io/crates/edf-rs)
[![docs.rs](https://img.shields.io/docsrs/edf-rs)](https://docs.rs/edf-rs/latest/edf_rs/)
![GitHub License](https://img.shields.io/github/license/BitFlaker/edf-rs)
![GitHub repo size](https://img.shields.io/github/repo-size/BitFlaker/edf-rs)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/BitFlaker/edf-rs)
![Version state](https://img.shields.io/badge/version_state-nightly-A04EED)

A pure Rust library for reading and writing EDF/EDF+ files. It aims at providing a feature-rich and reliable way to work with `*.edf` files. This crate is optimized to work efficiently with very large EDF file sizes. It is based on the official specification [here](https://www.edfplus.info/). Currently the primary focus for this project is to power my other project [NoctiG Scorer](https://github.com/BitFlaker/noctig-scorer).

**This library is an unofficial implementation. It is still in an early development stage which is not yet considered stable and will have breaking changes in the future.**

## ✨ Features
Here is a non-exhaustive list of all implemented and planned features for this library:

▉ &nbsp; Reading EDF/EDF+ files \
▉ &nbsp; Creating / Updating existing EDF/EDF+ files \
▉ &nbsp; Adding / Removing / Updating existing records and signals \
▉ &nbsp; Support for seeking \
▉ &nbsp; Reading data by custom duration (nanoseconds, seconds, etc.) \
▒ &nbsp; Extensive documentation \
▒ &nbsp; Examples \
┊ &nbsp; Support for BDF/BDF+ files and EDF extensions \
┊ &nbsp; Conversion from (and maybe to) other formats (e.g. [OpenBCI Recordings](https://docs.openbci.com/Software/OpenBCISoftware/GUIDocs/#exported-data)) \
┊ &nbsp; Additional features (e.g. merging files, etc.)

----
&nbsp;&nbsp;&nbsp; ▉ &nbsp;Implemented&nbsp;&nbsp;&nbsp; ▒ &nbsp;In progress&nbsp;&nbsp;&nbsp; ┊ &nbsp;Planned

----

## 🚀 Usage
The code snippet below shows how to open an EDF+ file, print the metadata and read the first few data-records.

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

## ⚠️ Disclaimer
I (the developer of this tool) am not a scientist, doctor or similar. I am just a programmer who maintains this tool as a hobby because it is the application I wish existed. This means it is possible that some features of this tool do not work as they should (due to lack of scientific knowledge or similar). This tool is not intended for medical treatment or diagnosis. This software is offered "as is" and it could contain errors, bugs or vulnerabilities which could lead to unexpected or undesirable consequences. If you encounter such problems, feel free to report them in the [issues](https://github.com/BitFlaker/edf-rs/issues) section. Keep in mind that this application is still in a very early development stage and not yet considered stable. I cannot and do not accept any liability for damages related to the use of this software. Use it at your own risk.

## 👥 Contributing
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as below, without any additional terms or conditions.

## 📜 License
This project is licensed under either of

* Apache License, Version 2.0 [[LICENSE-APACHE](LICENSE-APACHE)]
* MIT License [[LICENSE-MIT](LICENSE-MIT)]

at your option.
