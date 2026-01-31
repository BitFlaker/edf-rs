use std::error::Error;

#[derive(Debug)]
pub enum EDFError {
    InvalidUserIdSegmentCount,
    InvalidUserIdDate,
    InvalidUType,
    UserIdTooLong,
    InvalidRecordingIdSegmentCount,
    InvalidRecordingIdDate,
    RecordingIdTooLong,
    InvalidStartDate,
    InvalidStartTime,
    InvalidHeaderSize,
    InvalidRecordCount,
    InvalidRecordDuration,
    InvalidSignalCount,
    InvalidPhysicalRange,
    InvalidDigitalRange,
    InvalidSamplesCount,
    InvalidASCII,
    IllegalCharacters,
    FieldSizeExceeded,
    SignalNotAnnotation,
    InvalidHeaderTAL,
    MissingAnnotations,
    FileReadError(std::io::Error),
    FileWriteError(std::io::Error),
    InvalidReadRange,
    ReadWhileRecording,
    FileAlreadyExists,
    ItemNotFound,
    IndexOutOfBounds,
    InvalidRecordSignals,
}

impl Error for EDFError {}

impl std::fmt::Display for EDFError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "An error occurred during serialization/deserialization of the EDF file: {}",
            self
        )
    }
}
