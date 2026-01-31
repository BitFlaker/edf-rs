use chrono::NaiveDate;
use std::fmt::Display;
use std::str::FromStr;

use crate::EDFSpecifications;
use crate::error::edf_error::EDFError;
use crate::utils::{deserialize_field, is_printable_ascii, serialize_field};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PatientId {
    pub code: Option<String>,
    pub sex: Option<Sex>,
    pub date: Option<NaiveDate>,
    pub name: Option<String>, // Common field
    pub additional: Vec<Option<String>>,
}

impl PatientId {
    // TODO: This could be a deserializer option to try and parse the individual parts anyways
    /// Deserializes the provided user identification to the parsed struct. If the file type
    /// is not compatible with the EDF+ specification, the fields value will be stored in its
    /// entirety within the `name` field. This is to prevent splitting the field in a potentially
    /// undesired way.
    pub fn deserialize(value: String, spec: &EDFSpecifications) -> Result<Self, EDFError> {
        let parts = value.split_ascii_whitespace().collect::<Vec<_>>();

        // Parse user id based on EDF+ spec if it is valid
        if *spec == EDFSpecifications::EDFPlus && parts.len() >= 4 {
            return Ok(PatientId {
                code: deserialize_field(parts[0]),
                sex: deserialize_field(parts[1])
                    .map(|v| Sex::from_str(&v))
                    .transpose()?,
                date: deserialize_field(parts[2])
                    .map(|v| NaiveDate::parse_from_str(&v, "%d-%b-%Y"))
                    .transpose()
                    .map_err(|_| EDFError::InvalidUserIdDate)?,
                name: deserialize_field(parts[3]),
                additional: parts[4..].iter().cloned().map(deserialize_field).collect(),
            });
        }

        // Parse user id based on EBasic spec
        if *spec == EDFSpecifications::EDF {
            let mut user = PatientId::default();
            user.name = if value.is_empty() { None } else { Some(value) };
            return Ok(user);
        }

        Err(EDFError::InvalidUserIdSegmentCount)
    }

    pub fn serialize(&self, spec: &EDFSpecifications) -> Result<String, EDFError> {
        let value = match spec {
            EDFSpecifications::EDF => self.name.clone().unwrap_or_default(),
            EDFSpecifications::EDFPlus => {
                let code = serialize_field(self.code.clone());
                let u_type = serialize_field(self.sex.as_ref().map(|t| t.to_string()));
                let date = serialize_field(
                    self.date
                        .map(|d| d.format("%d-%b-%Y").to_string().to_uppercase()),
                );
                let name = serialize_field(self.name.clone());

                // Serialize additional fields and prefix with space if there is additional data
                let mut additional = self
                    .additional
                    .clone()
                    .into_iter()
                    .map(serialize_field)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !additional.is_empty() {
                    additional = format!(" {}", additional);
                }

                format!("{} {} {} {}{}", code, u_type, date, name, additional)
            }
        };

        // Ensure the header length does not exceed the maximum
        if value.len() > 80 {
            return Err(EDFError::UserIdTooLong);
        }

        // Ensure the serialized value only contains valid printable ASCII characters
        if !is_printable_ascii(&value) {
            return Err(EDFError::InvalidASCII);
        }

        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sex {
    Female,
    Male,
}

impl Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Female => write!(f, "F"),
            Self::Male => write!(f, "M"),
        }
    }
}

impl FromStr for Sex {
    type Err = EDFError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "F" => Ok(Self::Female),
            "M" => Ok(Self::Male),
            _ => Err(EDFError::InvalidUType),
        }
    }
}
