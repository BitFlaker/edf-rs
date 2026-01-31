use chrono::NaiveDate;

use crate::EDFSpecifications;
use crate::error::edf_error::EDFError;
use crate::utils::{deserialize_field, is_printable_ascii, serialize_field};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RecordingId {
    pub startdate: Option<NaiveDate>,
    pub admin_code: Option<String>,     // Common field
    pub technician: Option<String>,
    pub equipment: Option<String>,
    pub additional: Vec<Option<String>>
}

impl RecordingId {
    // TODO: This could be a deserializer option to try and parse the individual parts anyways
    /// Deserializes the provided recording identification to the parsed struct. If the file type
    /// is not compatible with the EDF+ specification, the fields value will be stored in its
    /// entirety within the `admin_code` field. This is to prevent splitting the field in a potentially
    /// undesired way.
    pub fn deserialize(value: String, spec: &EDFSpecifications) -> Result<Self, EDFError> {
        let parts = value.split_ascii_whitespace().collect::<Vec<_>>();

        // Parse patient id based on EDF+ spec if it is valid
        if *spec == EDFSpecifications::EDFPlus && parts.len() >= 5 && parts[0] == "Startdate" {
            return Ok(RecordingId {
                startdate: deserialize_field(parts[1])
                    .map(|v| NaiveDate::parse_from_str(&v, "%d-%b-%Y"))
                    .transpose()
                    .map_err(|_| EDFError::InvalidRecordingIdDate)?,
                admin_code: deserialize_field(parts[2]),
                technician: deserialize_field(parts[3]),
                equipment: deserialize_field(parts[4]),
                additional: parts[5..].iter().cloned().map(deserialize_field).collect()
            });
        }

        // Parse patient id based on EDF spec
        if *spec == EDFSpecifications::EDF {
            let mut recording = RecordingId::default();
            recording.admin_code = if value.is_empty() { None } else { Some(value) };
            return Ok(recording);
        }

        Err(EDFError::InvalidRecordingIdSegmentCount)
    }

    pub fn serialize(&self, spec: &EDFSpecifications) -> Result<String, EDFError> {
        let value = match spec {
            EDFSpecifications::EDF => self.admin_code.clone().unwrap_or_default(),
            EDFSpecifications::EDFPlus => {
                let startdate = serialize_field(self.startdate.map(|d| d.format("%d-%b-%Y").to_string().to_uppercase()));
                let admin_code = serialize_field(self.admin_code.clone());
                let technician = serialize_field(self.technician.clone());
                let equipment = serialize_field(self.equipment.clone());

                // Serialize additional fields and prefix with space if there is additional data
                let mut additional = self.additional.clone().into_iter().map(serialize_field).collect::<Vec<_>>().join(" ");
                if !additional.is_empty() {
                    additional = format!(" {}", additional);
                }

                format!("Startdate {} {} {} {}{}", startdate, admin_code, technician, equipment, additional)
            }
        };

        // Ensure the header length does not exceed the maximum
        if value.len() > 80 {
            return Err(EDFError::RecordingIdTooLong);
        }

        // Ensure the serialized value only contains valid printable ASCII characters
        if !is_printable_ascii(&value) {
            return Err(EDFError::InvalidASCII);
        }

        Ok(value)
    }
}
