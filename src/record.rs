use std::collections::HashMap;

use crate::error::edf_error::EDFError;
use crate::headers::annotation_list::AnnotationList;
use crate::headers::edf_header::EDFHeader;
use crate::headers::signal_header::SignalHeader;
use crate::save::{SaveInstruction, SaveValue};

#[derive(Debug, Default, Clone, PartialEq)]
struct RecordLayout {
    signal_map: HashMap<usize, SignalType>,
    annotation_samples_count: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
enum SignalType {
    Samples(usize),
    Annotation(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    layout: RecordLayout,
    pub(crate) default_offset: f64,
    pub raw_signal_samples: Vec<Vec<i16>>,
    pub annotations: Vec<Vec<AnnotationList>>,
}

impl Record {
    pub fn new(signal_headers: &Vec<SignalHeader>) -> Self {
        let mut raw_signal_samples = Vec::new();
        let mut annotations = Vec::new();
        let mut annotation_samples_count = Vec::new();
        let mut signal_map = HashMap::new();
        for (i, signal) in signal_headers.iter().enumerate() {
            if signal.is_annotation() {
                signal_map.insert(i, SignalType::Annotation(annotations.len()));
                annotation_samples_count.push(signal.samples_count);
                annotations.push(Vec::new());
            } else {
                signal_map.insert(i, SignalType::Samples(raw_signal_samples.len()));
                raw_signal_samples.push(vec![0; signal.samples_count]);
            }
        }

        Self {
            layout: RecordLayout {
                signal_map,
                annotation_samples_count,
            },
            default_offset: 0.0,
            raw_signal_samples,
            annotations,
        }
    }

    pub fn patch_record(&mut self, instructions: &Vec<SaveInstruction>) -> Result<(), EDFError> {
        if instructions.is_empty() {
            return Ok(());
        }

        // Process each instruction
        let mut signal_idx = instructions[0].index();
        let mut instruction_idx = 0;
        loop {
            let Some(tr) = instructions.get(instruction_idx) else {
                break;
            };

            match tr {
                SaveInstruction::Remove(idx) if *idx == signal_idx => {
                    instruction_idx += 1;
                    self.remove_signal(*idx)?;
                }
                SaveInstruction::Insert(idx, SaveValue::Signal(value)) if *idx == signal_idx => {
                    instruction_idx += 1;
                    if value.is_annotation() {
                        self.insert_annotation(*idx, value.samples_count)?;
                    } else {
                        self.insert_signal_samples(*idx, value.samples_count)?;
                    }
                }
                SaveInstruction::Update(idx, SaveValue::Signal(value)) if *idx == signal_idx => {
                    signal_idx += 1;
                    instruction_idx += 1;
                    self.update_samples_count(*idx, value.samples_count)?;
                }
                _ => {
                    signal_idx += 1;
                }
            }
        }

        Ok(())
    }

    pub fn insert_signal_samples(
        &mut self,
        signal_index: usize,
        samples_count: usize,
    ) -> Result<(), EDFError> {
        // Get count of signal indices which are samples and lower than the target index
        let insert_idx = (0..signal_index)
            .filter(|i| {
                self.layout
                    .signal_map
                    .get(&i)
                    .is_some_and(|s| matches!(s, SignalType::Samples(idx) if *idx < signal_index))
            })
            .count();

        // Increase the global signal index pointers in the signal map as well as the sample signal index pointers
        self.apply_index_change_samples(signal_index, insert_idx, 1);
        self.layout
            .signal_map
            .insert(signal_index, SignalType::Samples(insert_idx));

        // Insert the new annotation signal values
        self.raw_signal_samples
            .insert(insert_idx, vec![0; samples_count]);

        Ok(())
    }

    pub fn insert_annotation(
        &mut self,
        signal_index: usize,
        samples_count: usize,
    ) -> Result<(), EDFError> {
        // Get count of signal indices which are annotations and lower than the target index
        let insert_idx = (0..signal_index)
            .filter(|i| {
                self.layout.signal_map.get(&i).is_some_and(
                    |s| matches!(s, SignalType::Annotation(idx) if *idx < signal_index),
                )
            })
            .count();

        // Increase the global signal index pointers in the signal map as well as the annotation signal index pointers
        self.apply_index_change_annotation(signal_index, insert_idx, 1);
        self.layout
            .signal_map
            .insert(signal_index, SignalType::Annotation(insert_idx));

        // Insert the new annotation signal values
        self.layout
            .annotation_samples_count
            .insert(insert_idx, samples_count);
        self.annotations.insert(insert_idx, Vec::new());

        Ok(())
    }

    pub fn remove_signal(&mut self, signal_index: usize) -> Result<(), EDFError> {
        match self.layout.signal_map.remove(&signal_index) {
            Some(SignalType::Samples(idx)) => {
                self.raw_signal_samples.remove(idx);
                self.apply_index_change_samples(signal_index, idx, -1);
            }
            Some(SignalType::Annotation(idx)) => {
                self.layout.annotation_samples_count.remove(idx);
                self.annotations.remove(idx);
                self.apply_index_change_annotation(signal_index, idx, -1);
            }
            _ => return Err(EDFError::ItemNotFound),
        }

        Ok(())
    }

    pub fn update_samples_count(
        &mut self,
        signal_index: usize,
        samples_count: usize,
    ) -> Result<(), EDFError> {
        match self.layout.signal_map.get(&signal_index) {
            Some(SignalType::Samples(idx)) => {
                if let Some(count) = self.raw_signal_samples.get_mut(*idx) {
                    count.resize(samples_count, 0);
                } else {
                    return Err(EDFError::ItemNotFound);
                }
            }
            Some(SignalType::Annotation(idx)) => {
                if let Some(count) = self.layout.annotation_samples_count.get_mut(*idx) {
                    *count = samples_count;
                } else {
                    return Err(EDFError::ItemNotFound);
                }
            }
            _ => return Err(EDFError::ItemNotFound),
        }

        Ok(())
    }

    pub fn set_annotation(
        &mut self,
        signal_index: usize,
        annotations: Vec<AnnotationList>,
    ) -> Result<(), EDFError> {
        let Some(SignalType::Annotation(idx)) = self.layout.signal_map.get(&signal_index) else {
            return Err(EDFError::ItemNotFound);
        };

        let Some(old_annotations) = self.annotations.get_mut(*idx) else {
            return Err(EDFError::ItemNotFound);
        };

        *old_annotations = annotations;

        Ok(())
    }

    pub fn set_samples(&mut self, signal_index: usize, samples: Vec<i16>) -> Result<(), EDFError> {
        let Some(SignalType::Samples(idx)) = self.layout.signal_map.get(&signal_index) else {
            return Err(EDFError::ItemNotFound);
        };

        let Some(old_samples) = self.raw_signal_samples.get_mut(*idx) else {
            return Err(EDFError::ItemNotFound);
        };

        if old_samples.len() != samples.len() {
            return Err(EDFError::InvalidSamplesCount);
        }

        *old_samples = samples;

        Ok(())
    }

    pub fn get_digital_samples(&self, signal: &SignalHeader) -> Vec<Vec<i32>> {
        self.raw_signal_samples.iter().map(|signals| {
            signals.iter().map(|sample| {
                (*sample as i32).clamp(signal.digital_minimum, signal.digital_maximum)
            }).collect()
        }).collect()
    }

    pub fn get_physical_samples(&self, signal: &SignalHeader) -> Vec<Vec<f64>> {
        let range = (signal.physical_maximum - signal.physical_minimum) / (signal.digital_maximum - signal.digital_minimum) as f64;
        let offset = signal.physical_maximum / range - signal.digital_maximum as f64;

        self.raw_signal_samples.iter().map(|signals| {
            signals.iter().map(|sample| {
                let digital = *sample as f64;
                let physical = range * (offset + digital);
                physical.clamp(signal.physical_minimum, signal.physical_maximum)
            }).collect()
        }).collect()
    }

    fn apply_index_change_annotation(
        &mut self,
        signal_index: usize,
        target_index: usize,
        direction: i8,
    ) {
        let mut new = HashMap::new();
        for (k, v) in self.layout.signal_map.drain() {
            let new_global_index =
                (k as i64 + direction as i64 * (k >= signal_index) as i64) as usize;
            let value = if let SignalType::Annotation(idx) = v
                && idx >= target_index
            {
                SignalType::Annotation((idx as i64 + direction as i64) as usize)
            } else {
                v
            };
            new.insert(new_global_index, value);
        }
        self.layout.signal_map = new;
    }

    fn apply_index_change_samples(
        &mut self,
        signal_index: usize,
        target_index: usize,
        direction: i8,
    ) {
        let mut new = HashMap::new();
        for (k, v) in self.layout.signal_map.drain() {
            let new_global_index =
                (k as i64 + direction as i64 * (k >= signal_index) as i64) as usize;
            let value = if let SignalType::Samples(idx) = v
                && idx >= target_index
            {
                SignalType::Samples((idx as i64 + direction as i64) as usize)
            } else {
                v
            };
            new.insert(new_global_index, value);
        }
        self.layout.signal_map = new;
    }

    /// Returns the onset of the current record relative to the start of the recording of the EDF+ file.
    /// This only returns useful information for EDF+ files. Regular EDF files will always return the
    /// index of the data-record multiplied by the data-record duration as records are missing the time keeping context.
    /// If there were to be multiple signals labeled `EDF Annotations`, the first one will be used to check for the
    /// Time-keeping-list entry
    pub fn get_start_offset(&self) -> f64 {
        self.annotations
            .first()
            .map(|tals| tals.iter().find(|a| a.is_time_keeping()).map(|a| a.onset))
            .flatten()
            .unwrap_or(self.default_offset)
    }

    pub fn serialize(&self) -> Result<Vec<u8>, EDFError> {
        let mut result_buffer = vec![];

        for signal_idx in 0..self.layout.signal_map.len() {
            match self.layout.signal_map.get(&signal_idx) {
                Some(SignalType::Annotation(idx)) => {
                    if let Some(annotation) = self.annotations.get(*idx)
                        && let Some(sample_count) = self.layout.annotation_samples_count.get(*idx)
                    {
                        let tals = annotation
                            .iter()
                            .map(|a| a.serialize())
                            .collect::<Vec<_>>()
                            .join("");
                        let mut tal_bytes = tals.as_bytes().to_vec();
                        tal_bytes.extend(vec![0; 2 * sample_count - tal_bytes.len()]);
                        result_buffer.extend(tal_bytes);
                    }
                }
                Some(SignalType::Samples(idx)) => {
                    if let Some(signal) = self.raw_signal_samples.get(*idx) {
                        result_buffer.extend(
                            &signal
                                .into_iter()
                                .map(|s| s.to_le_bytes())
                                .flatten()
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                _ => {
                    panic!("Invalid record signal mapping index. This should not be possible")
                }
            }
        }

        Ok(result_buffer)
    }

    pub fn matches_signals(&self, signal_headers: &Vec<SignalHeader>) -> bool {
        // Validate the signal count of the record matches the provided signal header count
        let actual_count = self.annotations.len() + self.raw_signal_samples.len();
        if actual_count != signal_headers.len()
            || actual_count != self.layout.signal_map.len()
            || actual_count
                != self
                    .layout
                    .signal_map
                    .keys()
                    .max()
                    .map(|k| *k + 1)
                    .unwrap_or(0)
        {
            return false;
        }

        // Validate the sample count of every signal in the record matches the provided signal header
        for i in 0..actual_count {
            match self.layout.signal_map.get(&i) {
                Some(SignalType::Samples(idx)) => {
                    if !self
                        .raw_signal_samples
                        .get(*idx)
                        .is_some_and(|s| s.len() == signal_headers[i].samples_count)
                    {
                        return false;
                    }
                }
                Some(SignalType::Annotation(idx)) => {
                    if !self
                        .layout
                        .annotation_samples_count
                        .get(*idx)
                        .is_some_and(|c| *c == signal_headers[i].samples_count)
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }

        true
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RelativeRecordData {
    pub offset: f64,
    pub raw_signal_samples: Vec<i16>,
}

impl RelativeRecordData {
    pub fn new(offset: f64) -> Self {
        Self {
            offset,
            raw_signal_samples: Vec::new(),
        }
    }

    pub fn get_digital_samples(&self, signal: &SignalHeader) -> Vec<i32> {
        self.raw_signal_samples.iter().map(|sample| {
            (*sample as i32).clamp(signal.digital_minimum, signal.digital_maximum)
        }).collect()
    }

    pub fn get_physical_samples(&self, signal: &SignalHeader) -> Vec<f64> {
        let range = (signal.physical_maximum - signal.physical_minimum) / (signal.digital_maximum - signal.digital_minimum) as f64;
        let offset = signal.physical_maximum / range - signal.digital_maximum as f64;

        self.raw_signal_samples.iter().map(|sample| {
            let digital = *sample as f64;
            let physical = range * (offset + digital);
            physical.clamp(signal.physical_minimum, signal.physical_maximum)
        }).collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SpanningRecord {
    pub raw_signal_samples: Vec<Vec<RelativeRecordData>>,
    pub annotations: Vec<Vec<AnnotationList>>,
}

impl SpanningRecord {
    pub fn new(header: &EDFHeader) -> Self {
        let signal_count = header.signals.iter().filter(|s| !s.is_annotation()).count();
        Self {
            raw_signal_samples: vec![Vec::new(); signal_count],
            annotations: Vec::new(),
        }
    }

    pub fn is_spanning_wait(&self) -> bool {
        self.raw_signal_samples
            .iter()
            .all(|sp| sp.last().is_some_and(|data| data.raw_signal_samples.is_empty()))
    }

    pub fn remove_last_spanning_wait(&mut self) -> bool {
        if self.is_spanning_wait() {
            for signal in &mut self.raw_signal_samples {
                signal.remove(signal.len() - 1);
            }

            return true;
        }
        return false;
    }

    pub fn insert_spanning_wait(&mut self, offset: f64) {
        self.remove_last_spanning_wait();

        // Check if the last spanning entry is the same offset
        if self
            .raw_signal_samples
            .first()
            .map(|s| s.last())
            .flatten()
            .is_some_and(|s| s.offset == offset)
        {
            return;
        }

        // In case it is a new offset, insert the new spanning values
        for signal in &mut self.raw_signal_samples {
            signal.push(RelativeRecordData::new(offset));
        }
    }

    pub fn finish(&mut self) {
        self.remove_last_spanning_wait();
        // TODO: This should probably also go through all annotations and remove all
        // Time-keeping entries.
    }

    pub fn extend_samples(&mut self, signal_index: usize, samples: Vec<i16>) {
        if let Some(signal) = self.raw_signal_samples.get_mut(signal_index) {
            if let Some(data) = signal.last_mut() {
                data.raw_signal_samples.extend(samples);
            }
        }
    }
}
