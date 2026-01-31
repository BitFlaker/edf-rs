use chrono::{Datelike, NaiveDate, NaiveTime};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Seek, SeekFrom};
use std::str::FromStr;

use crate::EDFSpecifications;
use crate::error::edf_error::EDFError;
use crate::headers::patient::PatientId;
use crate::headers::recording::RecordingId;
use crate::headers::signal_header::SignalHeader;
use crate::record::Record;
use crate::utils::is_printable_ascii;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EDFHeader {
    pub(crate) version: String,
    pub(crate) patient_id: PatientId,
    pub(crate) recording_id: RecordingId,
    start_date: NaiveDate,
    pub(crate) start_time: NaiveTime,
    pub(crate) header_bytes: usize,
    pub(crate) specification: EDFSpecifications,
    pub(crate) is_continuous: bool,
    pub(crate) record_count: Option<usize>,
    pub(crate) record_duration: f64,
    pub(crate) signal_count: usize,
    pub(crate) signals: Vec<SignalHeader>,
    pub(crate) updated_signals: Option<Vec<SignalHeader>>,

    initial_record_size: usize,
    initial_record_hash: String,

    #[allow(dead_code)]
    reserved: String,
}

impl EDFHeader {
    pub fn new() -> Self {
        Self {
            version: "0".to_string(),
            updated_signals: None,
            ..Default::default()
        }
    }

    pub fn with_version(&mut self, version: String) -> &mut Self {
        self.version = version;
        self
    }

    pub fn with_patient_id(&mut self, patient_id: PatientId) -> &mut Self {
        self.patient_id = patient_id;
        self
    }

    pub fn with_recording_id(&mut self, recording_id: RecordingId) -> &mut Self {
        self.recording_id = recording_id;
        self
    }

    pub fn with_start_date(&mut self, start_date: NaiveDate) -> &mut Self {
        self.start_date = start_date;
        // TODO: Also update the start date in recording id
        self
    }

    pub fn with_start_time(&mut self, start_time: NaiveTime) -> &mut Self {
        self.start_time = start_time;
        self
    }

    pub fn with_specification(&mut self, specification: EDFSpecifications) -> &mut Self {
        self.specification = specification;
        self.is_continuous = self.specification == EDFSpecifications::EDF || self.is_continuous;
        self
    }

    pub fn with_is_continuous(&mut self, is_continuous: bool) -> &mut Self {
        self.is_continuous = is_continuous;
        self
    }

    pub fn with_record_count(&mut self, record_count: usize) -> &mut Self {
        self.record_count = Some(record_count);
        self
    }

    pub fn with_record_duration(&mut self, record_duration: f64) -> &mut Self {
        self.record_duration = record_duration;
        self
    }

    pub fn get_version(&self) -> &String {
        &self.version
    }

    pub fn get_patient_id(&self) -> &PatientId {
        &self.patient_id
    }

    pub fn get_recording_id(&self) -> &RecordingId {
        &self.recording_id
    }

    pub fn get_start_date(&self) -> NaiveDate {
        self.start_date
        // TODO: Also take the start date from recording id into consideration
    }

    pub fn get_start_time(&self) -> NaiveTime {
        self.start_time
    }

    pub fn get_header_bytes(&self) -> usize {
        self.header_bytes
    }

    pub fn get_specification(&self) -> EDFSpecifications {
        self.specification.clone()
    }

    pub fn is_continuous(&self) -> bool {
        self.is_continuous
    }

    pub fn get_record_count(&self) -> Option<usize> {
        self.record_count
    }

    pub fn get_record_duration(&self) -> f64 {
        self.record_duration
    }

    pub fn get_signals(&self) -> &Vec<SignalHeader> {
        self.updated_signals.as_ref().unwrap_or(&self.signals)
    }

    pub fn calculate_header_bytes(&self) -> usize {
        let signal_count = self.signals.len();
        let fixed_size = 8 + 80 + 80 + 8 + 8 + 8 + 44 + 8 + 8 + 4;
        let signal_size = 16 + 80 + 8 + 8 + 8 + 8 + 8 + 80 + 8 + 32;
        fixed_size + signal_count * signal_size
    }

    pub fn data_record_bytes(&self) -> usize {
        self.signals.iter().map(|s| s.samples_count * 2).sum()
    }

    pub fn get_signal_sample_frequency(&self, signal_index: usize) -> Option<f64> {
        self.signals
            .get(signal_index)
            .map(|s| s.samples_count as f64 / self.record_duration)
    }

    /// Returns the length of a data-record at the time the file was opened in bytes. This value
    /// is only required for saving files to get an accurate offset.
    pub(crate) fn get_initial_record_bytes(&self) -> usize {
        if self.initial_record_size == 0 {
            return self.data_record_bytes();
        }
        self.initial_record_size
    }

    /// Updates the length of a data-record at the time the file was opened. This is supposed to only
    /// be called after the file was saved and the header has changed on disk. This value
    /// is only required for saving files to get an accurate offset.
    pub(crate) fn update_initial_record_bytes(&mut self) {
        self.initial_record_size = self.data_record_bytes();
    }

    /// Returns SHA256 hash calculated when the file was opened. This value is only required for
    /// saving files to check whether or not the value of the header has changed.
    pub(crate) fn get_initial_header_sha256(&self) -> &String {
        &self.initial_record_hash
    }

    /// Updates the initial SHA256 hash calculated when the file was opened. This is supposed to only
    /// be called after the file was saved and the header has changed on disk. This value is only required for
    /// saving files to check whether or not the value of the header has changed.
    pub(crate) fn update_initial_header_sha256(&mut self) -> Result<(), EDFError> {
        Ok(self.initial_record_hash = self.get_sha256()?)
    }

    pub fn create_record(&self) -> Record {
        Record::new(self.updated_signals.as_ref().unwrap_or(&self.signals))
    }

    pub(crate) fn modify_signals(&mut self) -> &mut Vec<SignalHeader> {
        if self.updated_signals.is_none() {
            self.updated_signals = Some(self.signals.clone());
        }
        self.updated_signals.as_mut().unwrap()
    }

    pub fn serialize(&self) -> Result<String, EDFError> {
        let version = pad_string(&self.version, 8)?;
        let user_id = pad_string(&self.patient_id.serialize(&self.specification)?, 80)?;
        let recording_id = pad_string(&self.recording_id.serialize(&self.specification)?, 80)?;
        let start_date = pad_string(&Self::serialize_old_start_date(&self.start_date), 8)?;
        let start_time = pad_string(&self.start_time.format("%H.%M.%S").to_string(), 8)?;
        let reserved = pad_string(
            match self.specification {
                EDFSpecifications::EDF => "",
                EDFSpecifications::EDFPlus if self.is_continuous => "EDF+C",
                EDFSpecifications::EDFPlus => "EDF+D",
            },
            44,
        )?;
        let record_count = pad_string(
            &self
                .record_count
                .map(|c| c as i64)
                .unwrap_or(-1)
                .to_string(),
            8,
        )?;
        let record_duration = pad_string(&self.record_duration.to_string(), 8)?;
        let signal_count = pad_string(&self.signals.len().to_string(), 4)?;

        // Write general header values
        let mut header = format!(
            "{}{}{}{}{}{}{}{}{}",
            version,
            user_id,
            recording_id,
            start_date,
            start_time,
            // header_bytes (calculated at the bottom) [184..192]
            reserved,
            record_count,
            record_duration,
            signal_count
        );

        let signals = self.signals.clone();

        // Ensure an EDF+ file has at least 1 annotation signal
        if self.specification == EDFSpecifications::EDFPlus
            && !signals.iter().any(|s| s.is_annotation())
        {
            return Err(EDFError::MissingAnnotations);
        }

        // Set labels
        for signal in &signals {
            header += &pad_string(&signal.label, 16)?;
        }

        // Set transducers
        for signal in &signals {
            header += &pad_string(&signal.transducer, 80)?;
        }

        // Set physical dimensions
        for signal in &signals {
            header += &pad_string(&signal.physical_dimension, 8)?;
        }

        // Set physical minimum
        for signal in &signals {
            header += &pad_string(&signal.physical_minimum.to_string(), 8)?;
        }

        // Set physical maximum
        for signal in &signals {
            header += &pad_string(&signal.physical_maximum.to_string(), 8)?;
        }

        // Set digital minimum
        for signal in &signals {
            header += &pad_string(&signal.digital_minimum.to_string(), 8)?;
        }

        // Set digital maximum
        for signal in &signals {
            header += &pad_string(&signal.digital_maximum.to_string(), 8)?;
        }

        // Set pre-filters
        for signal in &signals {
            header += &pad_string(&signal.prefilter, 80)?;
        }

        // Set sample count per record
        for signal in &signals {
            header += &pad_string(&signal.samples_count.to_string(), 8)?;
        }

        // Set reserved fields
        for signal in &signals {
            header += &pad_string(&signal.reserved, 32)?;
        }

        // Get final header length and insert it into the header
        let header_bytes = header.len() + 8;
        header.insert_str(184, &pad_string(&header_bytes.to_string(), 8)?);

        // Ensure the serialized value only contains valid printable ASCII characters
        if !is_printable_ascii(&header) {
            return Err(EDFError::InvalidASCII);
        }

        Ok(header)
    }

    pub fn deserialize<R: BufRead + Seek>(reader: &mut R) -> Result<Self, EDFError> {
        // Immediately seek to the reserved location of the header to get the specification
        reader
            .seek(SeekFrom::Start(192))
            .map_err(EDFError::FileReadError)?;
        let reserved = read_ascii(reader, 44)?;

        // Distinguish between Pro and Basic specification
        let is_continuous_edfplus = reserved.starts_with("EDF+C");
        let is_discontinuous_edfplus = reserved.starts_with("EDF+D");
        let is_pro = is_continuous_edfplus || is_discontinuous_edfplus;
        let specification = if is_pro {
            EDFSpecifications::EDFPlus
        } else {
            EDFSpecifications::EDF
        };

        // Check if data is expected to be continuous based on header
        let is_continuous = is_continuous_edfplus || !is_pro;

        // Seek back to the beginning of the file and parse general header values
        reader
            .seek(SeekFrom::Start(0))
            .map_err(EDFError::FileReadError)?;
        let version = read_ascii(reader, 8)?.trim_ascii_end().to_string();
        let patient_id = PatientId::deserialize(
            read_ascii(reader, 80)?.trim_ascii_end().to_string(),
            &specification,
        )?;
        let recording_id = RecordingId::deserialize(
            read_ascii(reader, 80)?.trim_ascii_end().to_string(),
            &specification,
        )?;
        let start_date = Self::parse_old_start_date(&read_ascii(reader, 8)?)?;
        let start_time = NaiveTime::parse_from_str(&read_ascii(reader, 8)?, "%H.%M.%S")
            .map_err(|_| EDFError::InvalidStartTime)?;
        let header_bytes = usize::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
            .map_err(|_| EDFError::InvalidHeaderSize)?;

        // Skip the already parsed reserved field
        reader
            .seek(SeekFrom::Start(236))
            .map_err(EDFError::FileReadError)?;

        let record_count = usize::from_str(&read_ascii(reader, 8)?.trim_ascii_end()).ok();
        let record_duration = f64::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
            .map_err(|_| EDFError::InvalidRecordDuration)?; // Duration in seconds (should be whole number, except if data-record size would exceed 61440 bytes. The it should be smaller e.g. 0.01 (dot separator ALWAYS !))
        let signal_count = usize::from_str(&read_ascii(reader, 4)?.trim_ascii_end())
            .map_err(|_| EDFError::InvalidSignalCount)?;

        let mut signals = vec![SignalHeader::default(); signal_count];

        // Get labels
        for signal in &mut signals {
            signal.label = read_ascii(reader, 16)?.trim_ascii_end().to_string();
        }

        // Get transducers
        for signal in &mut signals {
            signal.transducer = read_ascii(reader, 80)?.trim_ascii_end().to_string();
        }

        // Get physical dimensions
        for signal in &mut signals {
            signal.physical_dimension = read_ascii(reader, 8)?.trim_ascii_end().to_string();
        }

        // Get physical minimum
        for signal in &mut signals {
            signal.physical_minimum = f64::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
                .map_err(|_| EDFError::InvalidPhysicalRange)?;
        }

        // Get physical maximum
        for signal in &mut signals {
            signal.physical_maximum = f64::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
                .map_err(|_| EDFError::InvalidPhysicalRange)?;
        }

        // Get digital minimum
        for signal in &mut signals {
            signal.digital_minimum = i32::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
                .map_err(|_| EDFError::InvalidPhysicalRange)?;
        }

        // Get digital maximum
        for signal in &mut signals {
            signal.digital_maximum = i32::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
                .map_err(|_| EDFError::InvalidPhysicalRange)?;
        }

        // Get pre-filters
        for signal in &mut signals {
            signal.prefilter = read_ascii(reader, 80)?.trim_ascii_end().to_string();
        }

        // Get sample count per record
        for signal in &mut signals {
            signal.samples_count = usize::from_str(&read_ascii(reader, 8)?.trim_ascii_end())
                .map_err(|_| EDFError::InvalidSamplesCount)?;
        }

        // Get reserved fields
        for signal in &mut signals {
            signal.reserved = read_ascii(reader, 32)?.trim_ascii_end().to_string();
        }

        let mut header = Self {
            version,
            patient_id,
            recording_id,
            start_date,
            start_time,
            header_bytes,
            reserved,
            specification,
            is_continuous,
            record_count,
            record_duration,
            signal_count,
            signals,
            initial_record_size: 0,
            initial_record_hash: String::new(),
            updated_signals: None,
        };

        // Get the hash of the header value to check for changes on save later
        header.initial_record_hash = header.get_sha256()?;
        header.update_initial_record_bytes();

        Ok(header)
    }

    /// Serializes the header of the EDF file and calculates a SHA256 hash and returns the result
    pub fn get_sha256(&self) -> Result<String, EDFError> {
        let serialized = self.serialize()?;
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    pub fn is_recording(&self) -> bool {
        self.record_count.is_none()
    }

    /// Returns the start date of the recording by returning the start date specified in `recording_id` or
    /// if it is not specified, using the old start-date value. Note that the old start date only supports the
    /// year range 1985 - 2084, a year outside this range will return the year 2100. This means if the start date
    /// is not specified within the `recording_id`, you might get an invalid date.
    pub fn start_date(&self) -> NaiveDate {
        self.recording_id.startdate.unwrap_or(self.start_date)
    }

    /// Returns the parsed old style date with clipping year 1985. When the year is later than 2084, the expected
    /// input year is the string 'yy' and this will return the NativeDate with year 2100. Input format has to be dd.mm.yy
    pub fn parse_old_start_date(date: &str) -> Result<NaiveDate, EDFError> {
        let parts = date.split('.').collect::<Vec<_>>();
        let year;

        // Ensure the date is three numbers separated by a dot
        if parts.len() != 3 {
            return Err(EDFError::InvalidStartDate);
        }

        // Check if the year is >2084 (year is 'yy') or in the range of 1985 and 2084
        if parts[2] == "yy" {
            year = "2100".to_string();
        } else if let Ok(year_num) = u8::from_str(parts[2]) {
            if year_num < 85 {
                year = format!("20{:0>2}", year_num);
            } else if year_num < 100 {
                year = format!("19{:0>2}", year_num);
            } else {
                return Err(EDFError::InvalidStartDate);
            }
        } else {
            return Err(EDFError::InvalidStartDate);
        }

        // Build the final year string to format dd.mm.yyyy
        let parsed_year = format!("{}.{}.{}", parts[0], parts[1], year);
        NaiveDate::parse_from_str(&parsed_year, "%d.%m.%Y").map_err(|_| EDFError::InvalidStartDate)
    }

    /// Returns the serialized old style date with clipping year 1985. When the year is later than 2084, the expected
    /// output year is the string 'yy'. The output format will be dd.mm.yy
    pub fn serialize_old_start_date(date: &NaiveDate) -> String {
        let year = if date.year() >= 2085 || date.year() <= 1984 {
            "yy".to_string()
        } else {
            format!("{:0>2}", (date.year() % 100))
        };

        // Build the final year string to format dd.mm.yyyy
        format!("{:0>2}.{:0>2}.{}", date.day(), date.month(), year)
    }
}

pub fn read_ascii<'a, R: BufRead>(reader: &'a mut R, count: usize) -> Result<String, EDFError> {
    let mut buf = vec![0; count];
    reader
        .read_exact(&mut buf)
        .map_err(EDFError::FileReadError)?;

    Ok(buf.iter().map(|c| *c as char).collect())
}

fn pad_string(value: &str, size: usize) -> Result<String, EDFError> {
    if value.len() > size {
        return Err(EDFError::FieldSizeExceeded);
    }
    let padding = " ".repeat(size - value.len());

    Ok(format!("{}{}", value, padding))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers::patient::PatientId;
    use crate::headers::patient::Sex;
    use crate::headers::recording::RecordingId;

    use chrono::{NaiveDate, NaiveTime};
    use std::io::BufReader;
    use std::io::Cursor;

    #[test]
    fn serialize() {
        let test_header = "0       MCH-0234567 F 16-SEP-1987 Haagse_Harry                                          Startdate 16-SEP-1987 PSG-1234/1987 NN Telemetry03                              16.09.8720.35.001024    EDF+C                                       2880    30      3   EEG Fpz-Cz      Temp rectal     EDF Annotations AgAgCl cup electrodes                                                           Rectal thermistor                                                                                                                                               uV      degC            -440    34.4    -1      510     40.2    1       -2048   -2048   -32768  2047    2047    32767   HP:0.1Hz LP:75Hz N:50Hz                                                         LP:0.1Hz (first order)                                                                                                                                          15000   3       320     Reserved for EEG signal         Reserved for Body temperature                                   ".to_string();
        let cursor = Cursor::new(test_header.as_bytes());
        let mut reader = BufReader::new(cursor);
        let value = EDFHeader::deserialize(&mut reader);
        assert!(value.is_ok());
        let value = value.unwrap();
        let serialized = value.serialize();
        assert!(serialized.is_ok());
        assert_eq!(serialized.unwrap(), test_header);
    }

    #[test]
    fn deserialize() {
        let test_header = "0       MCH-0234567 F 16-SEP-1987 Haagse_Harry                                          Startdate 16-SEP-1987 PSG-1234/1987 NN Telemetry03                              16.09.8720.35.001024    EDF+C                                       2880    30      3   EEG Fpz-Cz      Temp rectal     EDF Annotations AgAgCl cup electrodes                                                           Rectal thermistor                                                                                                                                               uV      degC            -440    34.4    -1      510     40.2    1       -2048   -2048   -32768  2047    2047    32767   HP:0.1Hz LP:75Hz N:50Hz                                                         LP:0.1Hz (first order)                                                                                                                                          15000   3       320     Reserved for EEG signal         Reserved for Body temperature                                   ".to_string();
        let cursor = Cursor::new(test_header.as_bytes());
        let mut reader = BufReader::new(cursor);
        let value = EDFHeader::deserialize(&mut reader);
        let mut expected = EDFHeader {
            version: "0".to_string(),
            patient_id: PatientId {
                code: Some("MCH-0234567".to_string()),
                sex: Some(Sex::Female),
                date: Some(NaiveDate::from_ymd_opt(1987, 09, 16).unwrap()),
                name: Some("Haagse Harry".to_string()),
                additional: Vec::new(),
            },
            recording_id: RecordingId {
                startdate: Some(NaiveDate::from_ymd_opt(1987, 09, 16).unwrap()),
                admin_code: Some("PSG-1234/1987".to_string()),
                technician: Some("NN".to_string()),
                equipment: Some("Telemetry03".to_string()),
                additional: Vec::new(),
            },
            start_date: NaiveDate::from_ymd_opt(1987, 09, 16).unwrap(),
            start_time: NaiveTime::from_hms_opt(20, 35, 00).unwrap(),
            header_bytes: 1024,
            specification: EDFSpecifications::EDFPlus,
            is_continuous: true,
            record_count: Some(2880),
            record_duration: 30.0,
            signal_count: 3,
            signals: vec![
                SignalHeader {
                    label: "EEG Fpz-Cz".to_string(),
                    transducer: "AgAgCl cup electrodes".to_string(),
                    physical_dimension: "uV".to_string(),
                    physical_minimum: -440.0,
                    physical_maximum: 510.0,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefilter: "HP:0.1Hz LP:75Hz N:50Hz".to_string(),
                    samples_count: 15000,
                    reserved: "Reserved for EEG signal".to_string(),
                },
                SignalHeader {
                    label: "Temp rectal".to_string(),
                    transducer: "Rectal thermistor".to_string(),
                    physical_dimension: "degC".to_string(),
                    physical_minimum: 34.4,
                    physical_maximum: 40.2,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefilter: "LP:0.1Hz (first order)".to_string(),
                    samples_count: 3,
                    reserved: "Reserved for Body temperature".to_string(),
                },
                SignalHeader {
                    label: "EDF Annotations".to_string(),
                    transducer: "".to_string(),
                    physical_dimension: "".to_string(),
                    physical_minimum: -1.0,
                    physical_maximum: 1.0,
                    digital_minimum: -32768,
                    digital_maximum: 32767,
                    prefilter: "".to_string(),
                    samples_count: 320,
                    reserved: "".to_string(),
                },
            ],
            reserved: "EDF+C                                       ".to_string(),
            initial_record_size: 30646,
            updated_signals: None,
            initial_record_hash: String::new(),
        };
        assert!(expected.update_initial_header_sha256().is_ok());
        assert!(value.is_ok());
        let value = value.unwrap();
        assert_eq!(value, expected);
        assert_eq!(value.serialize().unwrap(), test_header);
    }
}
