/*!
`edf-rs` is a pure Rust library for reading and writing EDF/EDF+ files. It aims at providing a feature-rich and
reliable way to work with `*.edf` files. This crate is optimized to work efficiently with very large EDF file sizes.
It is based on the official specification [here](https://www.edfplus.info/). Currently the primary
focus for this project is to power my other project [NoctiG Scorer](https://github.com/BitFlaker/noctig-scorer).

**This library is an unofficial implementation. It is still in an early development stage which is not yet
considered stable and will have breaking changes in the future.**

# Examples
To get started using this crate, follow the examples below. It will outline how to create and read a
basic EDF+ file with some test data. To see all available fields and functions, take a look at the
individual module documentations.

## Create an EDF+ file

The following example shows how to create a new EDF+ file and fill it with test data. This will
generate a file called `recording.edf`. It will have 1 regular signal and 1 annotations signal.
The regular signal will contain 100 samples with values from 0 to 99 in every data-record. The
annotations signal will have a time-keeping annotation and an additional record-global annotation.

```no_run
use chrono::{NaiveDate, NaiveTime};

use edf_rs::EDFSpecifications;
use edf_rs::file::EDFFile;
use edf_rs::record::Record;
use edf_rs::headers::patient::{PatientId, Sex};
use edf_rs::headers::recording::RecordingId;
use edf_rs::headers::signal_header::SignalHeader;
use edf_rs::headers::annotation_list::AnnotationList;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the EDF+ file from any path
    let mut edf = EDFFile::new("recording.edf")?;

    // Configure the header of the EDF file
    edf.header
        .with_specification(EDFSpecifications::EDFPlus)
        .with_is_continuous(true)
        .with_patient_id(PatientId {
            code: Some("PAT-CODE1".to_string()),
            name: Some("PAT-NAME".to_string()),
            date: Some(NaiveDate::from_ymd_opt(2001, 07, 11).unwrap()),
            sex: Some(Sex::Male),
            additional: Vec::new(),
        })
        .with_recording_id(RecordingId {
            admin_code: Some("REC-CODE1".to_string()),
            equipment: Some("EQUIPMENT".to_string()),
            technician: Some("TECHNICIAN".to_string()),
            startdate: Some(NaiveDate::from_ymd_opt(2026, 02, 13).unwrap()),
            additional: Vec::new()
        })
        .with_start_date(NaiveDate::from_ymd_opt(2026, 02, 13).unwrap())
        .with_start_time(NaiveTime::from_hms_opt(17, 30, 0).unwrap())
        .with_record_duration(1.0);

    // Create a regular signal
    let mut signal = SignalHeader::new();
    signal.with_label("Signal".to_string())
        .with_transducer("AgAgCl cup electrodes".to_string())
        .with_physical_dimension("uV".to_string())
        .with_physical_range(-440.0, 510.0)
        .with_digital_range(-2048, 2047)
        .with_samples_count(100);

    // Insert the regular and the annotation signals
    edf.insert_signal(0, signal).unwrap();
    edf.insert_signal(1, SignalHeader::new_annotation(80)).unwrap();

    // Insert some data-records
    edf.append_record(generate_record(&edf, 0)).unwrap();
    edf.append_record(generate_record(&edf, 1)).unwrap();
    edf.append_record(generate_record(&edf, 2)).unwrap();
    edf.append_record(generate_record(&edf, 3)).unwrap();
    edf.append_record(generate_record(&edf, 4)).unwrap();

    // Save the file
    edf.save().unwrap();

    Ok(())
}

fn generate_record(edf: &EDFFile, index: usize) -> Record {
    let mut record = edf.header.create_record();
    record.signal_samples = vec![
        (0..100).collect()
    ];
    record.annotations = vec![vec![
        AnnotationList::new(0.0 + index as f64, 0.0, vec![
            "".to_string(),
            format!("GlobalAnnotation {}", index)
        ]).unwrap()
    ]];

    record
}
```

## Read an EDF+ file

The following example shows how to read an EDF+ file from the beginning to the end. It will read the EDF
file created with the example [above](#create-an-edf-file). It will read all data-records from the file and
print the header, all annotations and the maximum values of each signal in every data-record to the console.

```no_run
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

Further examples will be added in the future
*/

pub mod error;
pub mod file;
pub mod headers;
pub mod record;
pub mod save;
mod tests;
pub mod utils;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EDFSpecifications {
    /// The original EDF specification from 1992. See the official specifications [here](https://www.edfplus.info/specs/edf.html).
    EDF,

    #[default]
    /// The extended EDF specification from 2003. See the official specifications [here](https://www.edfplus.info/specs/edfplus.html).
    EDFPlus,
}
