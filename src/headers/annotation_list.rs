use std::str::FromStr;

use crate::error::edf_error::EDFError;

// In case of multiple annotation signals, only the first one is required to have TALTKs and it is the only one used as a ref. Others could have them too, but they would simply be ignored / counted as empty free text
// If annotation starts in e.g. DR 12 and has a duration until DR 16, it will only show as an annotation in DR 12 and not in any of DR 13, DR 14, etc.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnnotationList {
    pub onset: f64, // relative to file_start time
    pub duration: f64,
    pub annotations: Vec<String>,
}

impl AnnotationList {
    pub fn new(onset: f64, duration: f64, annotations: Vec<String>) -> Result<Self, EDFError> {
        if !annotations.iter().all(is_valid_string) {
            return Err(EDFError::IllegalCharacters);
        }

        Ok(Self {
            onset,
            duration,
            annotations,
        })
    }

    pub fn new_time_keeping(onset: f64) -> Self {
        Self {
            onset,
            duration: 0.0,
            annotations: vec![String::new()],
        }
    }

    pub fn new_time_keeping_reasoned(onset: f64, reason: String) -> Self {
        Self {
            onset,
            duration: 0.0,
            annotations: vec![String::new(), reason],
        }
    }

    pub fn add_annotation(&mut self, annotation: String) -> Result<(), EDFError> {
        self.insert_annotation(self.annotations.len(), annotation)
    }

    pub fn insert_annotation(&mut self, index: usize, annotation: String) -> Result<(), EDFError> {
        if !is_valid_string(&annotation) {
            return Err(EDFError::IllegalCharacters);
        }

        self.annotations.insert(index, annotation);

        Ok(())
    }

    pub fn remove_annotation(&mut self, index: usize) {
        self.annotations.remove(index);
    }

    pub fn get_annotations(&self) -> &Vec<String> {
        &self.annotations
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, EDFError> {
        // Trim padding NUL bytes from end and remove the last byte HEX 14 which follows the last annotation value.
        // Therefore splitting at byte HEX 14 later returns the correct amount of annotations.
        if !data.ends_with(&[b'\x14', b'\x00']) {
            return Err(EDFError::InvalidHeaderTAL);
        }
        let data = &data[..data.len() - 2];

        // Split the TAL header (ASCII) and annotations (UTF-8)
        let header: String = data
            .into_iter()
            .take_while(|c| **c != b'\x14')
            .map(|c| *c as char)
            .collect();

        // Get header values (separated by byte HEX 15)
        let header_parts = header.split('\x15').collect::<Vec<_>>();
        if header_parts.is_empty() {
            return Err(EDFError::InvalidHeaderTAL);
        }

        // Parse onset and duration from header
        let onset = f64::from_str(header_parts[0]).map_err(|_| EDFError::InvalidHeaderTAL)?;
        let duration = header_parts
            .iter()
            .nth(1)
            .map(|d| f64::from_str(*d))
            .transpose()
            .map_err(|_| EDFError::InvalidHeaderTAL)?
            .unwrap_or(0.0);

        // Parse annotations (skip header bytes)
        let data = &data[header.len() + 1..];
        let annotations = data
            .split(|c| *c == b'\x14')
            .map(|a| String::from_utf8_lossy(a).to_string())
            .collect::<Vec<_>>();

        Ok(AnnotationList {
            onset,
            duration,
            annotations,
        })
    }

    pub fn serialize(&self) -> String {
        if self.annotations.is_empty() {
            return String::new();
        }

        let onset_sign = if self.onset >= 0.0 { "+" } else { "-" };
        let onset = format!("{}{}", onset_sign, self.onset);
        let header = if self.duration <= 0.0 {
            format!("{}\x14", onset)
        } else {
            format!("{}\x15{}\x14", onset, self.duration)
        };

        let annotations = self.annotations.join("\x14");

        format!("{}{}\x14\x00", header, annotations)
    }

    pub fn is_time_keeping(&self) -> bool {
        self.annotations
            .first()
            .map(String::is_empty)
            .unwrap_or(false)
    }

    pub fn time_keeping_reason(&self) -> Option<String> {
        if !self.is_time_keeping() {
            return None;
        }

        self.annotations.iter().nth(1).cloned()
    }
}

fn is_valid_string(s: &String) -> bool {
    s.chars()
        .all(|c| !matches!(c, '\0'..='\x1f') || c == '\t' || c == '\n' || c == '\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let tal = AnnotationList::deserialize(b"+30\x1520\x14\x14\x00").unwrap();
        assert_eq!(tal.onset, 30.0);
        assert_eq!(tal.duration, 20.0);
        assert!(tal.is_time_keeping());
        assert_eq!(tal.annotations.len(), 1);

        let tal = AnnotationList::deserialize(b"+30\x14\x14\x00").unwrap();
        assert_eq!(tal.onset, 30.0);
        assert_eq!(tal.duration, 0.0);
        assert!(tal.is_time_keeping());
        assert_eq!(tal.annotations.len(), 1);

        let tal = AnnotationList::deserialize(b"+30\x14\x14");
        assert!(tal.is_err());

        let tal =
            AnnotationList::deserialize(b"-0.489\x158.123\x14\x14Some reason\x14\x00").unwrap();
        assert_eq!(tal.onset, -0.489);
        assert_eq!(tal.duration, 8.123);
        assert!(tal.is_time_keeping());
        assert_eq!(tal.annotations.len(), 2);
        assert_eq!(tal.annotations[1], "Some reason".to_string());

        let tal = AnnotationList::deserialize(b"+0\x14Free text\x14\x00").unwrap();
        assert_eq!(tal.onset, 0.0);
        assert_eq!(tal.duration, 0.0);
        assert!(!tal.is_time_keeping());
        assert_eq!(tal.annotations.len(), 1);
        assert_eq!(tal.annotations[0], "Free text".to_string());

        let tal = AnnotationList::deserialize(b"+30\x1520\x14\x14\x00").unwrap();
        assert_eq!(tal.annotations.len(), 1);
    }
}
