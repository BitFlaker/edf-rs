use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Cursor, Seek, SeekFrom, Write};
use std::iter::repeat_n;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use crate::EDFSpecifications;
use crate::error::edf_error::EDFError;
use crate::headers::annotation_list::AnnotationList;
use crate::headers::edf_header::EDFHeader;
use crate::headers::signal_header::SignalHeader;
use crate::record::{Record, SpanningRecord};
use crate::save::{SaveInstruction, SaveValue, normalize_instructions};
use crate::utils::take_vec;

/// The desired strategy to delete data-records with. This option only has an effect on EDF+ files and
/// not on regular EDF files. It determines whether or not to shift the timestamps of data-records
/// following a deleted data-record
#[derive(Debug, Default, Clone, PartialEq)]
pub enum RecordDeleteStrategy {
    /// In case a record was deleted in the middle of a recording, the timestamps of all
    /// records following the deleted one will be shifted forward by the duration of 1 data-record.
    /// Therefore EDF+ files will keep the `is_continuous` state they had before deleting.
    ///
    /// # Note
    /// This is not yet implemented and currently removes the record without adjusting the timestamps
    /// of following records while keeping `is_continuous` unchanged.
    Continuous,

    /// In case a record was deleted in the middle of a recording, the timestamps of all
    /// records following the deleted one will remain the same as they were before. This will
    /// create a time gap between two data-records. Therefore EDF+ files will become discontinuous
    /// if they were continuous before deleting
    #[default]
    Discontinuous,
}

/// The mode the EDF file is currently being edited in. It primarily handles the way the data-record count field
/// in the file header is being treated on save. This option will not have any effect on read-only operations
/// on an EDF file
#[derive(Debug, Default, Clone, PartialEq)]
pub enum SaveMode {
    /// This mode is supposed to be used for editing / creating EDF files which are not currently being recorded
    /// in a live setting. It primarily affects the way the header updates its data-record count field. The data-record
    /// count will always be updated to the
    #[default]
    Default,

    /// This mode is supposed to be used when the EDF file is constantly being updated due to currently
    /// recording data in a live setting. It primarily affects the way the header updates its data-record count
    /// field. While recording the count will remain at -1. Therefore after finishing the recording, the save mode
    /// has to be changed to `SaveMode::Default` and saved again. This ensures the correct data-record count
    /// is being saved after finishing with the recording
    Recording,
}

pub struct EDFFile {
    pub header: EDFHeader,
    path: PathBuf,
    reader: BufReader<File>,
    record_read_offset_ns: u128,
    instructions: Vec<SaveInstruction>,
    signal_instructions: Vec<SaveInstruction>,
    record_counter: usize,
    signal_counter: usize,
    record_delete_strategy: RecordDeleteStrategy,
    save_mode: SaveMode,
}

impl EDFFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, EDFError> {
        let file = File::open(&path).map_err(EDFError::FileReadError)?;
        let mut reader = BufReader::new(file);
        let header = EDFHeader::deserialize(&mut reader)?;

        Ok(Self {
            record_counter: header.record_count.unwrap_or(0),
            signal_counter: header.signal_count,
            path: path.as_ref().to_path_buf(),
            record_read_offset_ns: 0,
            signal_instructions: Vec::new(),
            instructions: Vec::new(),
            header,
            reader,
            record_delete_strategy: RecordDeleteStrategy::default(),
            save_mode: SaveMode::default(),
        })
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, EDFError> {
        // Ensure the provided file does not exist yet and create the empty file
        if path.as_ref().exists() {
            return Err(EDFError::FileAlreadyExists);
        }
        File::create(&path).map_err(EDFError::FileWriteError)?;

        let file = File::open(&path).map_err(EDFError::FileReadError)?;
        let reader = BufReader::new(file);
        let header = EDFHeader::new();

        Ok(Self {
            header,
            reader,
            path: path.as_ref().to_path_buf(),
            record_read_offset_ns: 0,
            signal_counter: 0,
            record_counter: 0,
            signal_instructions: Vec::new(),
            instructions: vec![SaveInstruction::WriteHeader],
            record_delete_strategy: RecordDeleteStrategy::default(),
            save_mode: SaveMode::default(),
        })
    }

    /// Updates the mode for the save strategy. Setting this value will cause an updated EDF file header
    /// on the next call of the `save()` function. See `SaveMode` for more details.
    pub fn set_save_mode(&mut self, mode: SaveMode) {
        self.save_mode = mode;
        self.instructions.insert(0, SaveInstruction::WriteHeader);
    }

    pub fn insert_signal(&mut self, index: usize, signal: SignalHeader) -> Result<(), EDFError> {
        let instruction = SaveInstruction::Insert(index, SaveValue::Signal(signal.clone()));
        self.header.modify_signals().insert(index, signal);

        // Patch all records in pending instructions
        self.patch_records_with_instruction(instruction.clone())?;

        // Add the instruction
        self.signal_counter += 1;
        self.signal_instructions.push(instruction);

        Ok(())
    }

    pub fn update_signal(&mut self, index: usize, signal: SignalHeader) -> Result<(), EDFError> {
        let instruction = SaveInstruction::Update(index, SaveValue::Signal(signal.clone()));
        self.header.modify_signals()[index] = signal;

        // Patch all records in pending instructions
        self.patch_records_with_instruction(instruction.clone())?;

        // Add the instruction
        self.signal_instructions.push(instruction);

        Ok(())
    }

    pub fn remove_signal(&mut self, index: usize) -> Result<(), EDFError> {
        if self.signal_counter <= index {
            return Err(EDFError::IndexOutOfBounds);
        }
        let instruction = SaveInstruction::Remove(index);
        self.header.modify_signals().remove(index);

        // Patch all records in pending instructions
        self.patch_records_with_instruction(instruction.clone())?;

        // Add the instruction
        self.signal_counter -= 1;
        self.signal_instructions.push(instruction);

        Ok(())
    }

    fn patch_records_with_instruction(
        &mut self,
        instruction: SaveInstruction,
    ) -> Result<(), EDFError> {
        let instruction_listed = vec![instruction];
        for record in self.instructions.iter_mut().filter_map(|i| match i {
            SaveInstruction::Append(SaveValue::Record(record))
            | SaveInstruction::Insert(_, SaveValue::Record(record))
            | SaveInstruction::Update(_, SaveValue::Record(record)) => Some(record),
            _ => None,
        }) {
            record.patch_record(&instruction_listed)?;
        }

        Ok(())
    }

    pub fn insert_record(&mut self, index: usize, record: Record) -> Result<(), EDFError> {
        if !record.matches_signals(self.header.get_signals()) {
            return Err(EDFError::InvalidRecordSignals);
        }

        self.record_counter += 1;
        self.instructions
            .push(SaveInstruction::Insert(index, SaveValue::Record(record)));

        Ok(())
    }

    pub fn update_record(&mut self, index: usize, record: Record) -> Result<(), EDFError> {
        if !record.matches_signals(self.header.get_signals()) {
            return Err(EDFError::InvalidRecordSignals);
        }

        self.instructions
            .push(SaveInstruction::Update(index, SaveValue::Record(record)));

        Ok(())
    }

    pub fn append_record(&mut self, record: Record) -> Result<(), EDFError> {
        if !record.matches_signals(self.header.get_signals()) {
            return Err(EDFError::InvalidRecordSignals);
        }

        self.record_counter += 1;
        self.instructions
            .push(SaveInstruction::Append(SaveValue::Record(record)));

        Ok(())
    }

    /// Removes the record at the given index. If the file is an EDF+ file, it will adjust the offset
    /// of data-records after the given record in case `record_delete_strategy` is `ShiftOffsets` (Therefore keeping continuous
    /// EDF+ files continuous). Otherwise it will remove the data-record without shifting the offset of subsequent
    /// data-records (Therefore making continuous EDF+ files discontinuous).
    pub fn remove_record(&mut self, index: usize) -> Result<(), EDFError> {
        if self.record_counter <= index {
            return Err(EDFError::IndexOutOfBounds);
        }

        self.record_counter -= 1;
        self.instructions.push(SaveInstruction::Remove(index));

        Ok(())
    }

    fn records_match_signals(&self) -> bool {
        !self
            .instructions
            .iter()
            .filter_map(|i| match i {
                SaveInstruction::Append(SaveValue::Record(record))
                | SaveInstruction::Insert(_, SaveValue::Record(record))
                | SaveInstruction::Update(_, SaveValue::Record(record)) => Some(record),
                _ => None,
            })
            .any(|record| !record.matches_signals(self.header.get_signals()))
    }

    pub fn save(&mut self) -> Result<(), EDFError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)
            .map_err(EDFError::FileWriteError)?;

        let initial_filesize = file.metadata().map_err(EDFError::FileWriteError)?.len();
        let initial_signal_count = self.header.signal_count;
        let initial_record_count = self.header.record_count.unwrap_or(0);
        let initial_signals = self.header.signals.clone();
        let initial_record_duration = self.header.record_duration;
        let initial_header_size = self.header.header_bytes as u64;
        let initial_record_bytes = self.header.get_initial_record_bytes();

        // Update all header values to match the new state

        // Update the record count if not currently recording
        if self.save_mode == SaveMode::Default {
            self.header.record_count = Some(self.record_counter);
        }

        // Set the new signals and update the signal count
        if let Some(updated) = self.header.updated_signals.take() {
            self.header.signals = updated;
        }
        self.header.signal_count = self.header.signals.len();

        // In case there are no signals, remove all records as they will all have a length of 0 bytes
        if self.header.signal_count == 0 {
            self.header.record_count = self.header.record_count.map(|_| 0);
        }

        // Calculate new header and record sizes
        self.header.header_bytes = self.header.calculate_header_bytes();
        let new_record_bytes = self.header.data_record_bytes();

        // Ensure WriteHeader is at max once (and at index 0) and automatically add it if the header changed and it is not yet present
        let header_size_diff = self.header.header_bytes as i64 - initial_header_size as i64;
        let header_changed =
            *self.header.get_initial_header_sha256() != self.header.get_sha256()?;
        let header_instruct_positions = self
            .instructions
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (*x == SaveInstruction::WriteHeader).then_some(i))
            .collect::<Vec<_>>();
        if header_instruct_positions.len() >= 1 && header_instruct_positions[0] != 0 {
            for i in header_instruct_positions.iter().rev() {
                self.instructions.remove(*i);
            }
            self.instructions.insert(0, SaveInstruction::WriteHeader);
        } else if header_instruct_positions.len() >= 1 {
            for i in header_instruct_positions.iter().skip(1).rev() {
                self.instructions.remove(*i);
            }
        } else if header_instruct_positions.is_empty() && header_changed {
            self.instructions.insert(0, SaveInstruction::WriteHeader);
        }

        // Try to get the current read position to go back to after saving
        let initial_read_position = self
            .reader
            .stream_position()
            .map_err(EDFError::FileWriteError)?;
        let initial_record_position = if initial_record_bytes == 0 {
            None
        } else {
            initial_read_position
                .checked_sub(initial_header_size)
                .map(|pos| pos % initial_record_bytes as u64)
        };

        // If there are no instructions at all, nothing has to be done and the input remains the same
        if self.instructions.is_empty() && self.signal_instructions.is_empty() {
            return Ok(());
        }

        // Transform the list of instructions into a simplified sorted list of instructions
        let instructions = normalize_instructions(&self.instructions, initial_record_count);
        let signal_instructions =
            normalize_instructions(&self.signal_instructions, initial_signal_count);

        // Ensure all records have the correct signal layout before writing anything to disk
        if !self.records_match_signals() {
            return Err(EDFError::InvalidRecordSignals);
        }

        // If there were instructions, but all cancelled out, nothing has to be done either and the
        // input remains the same again. e.g. Insert at index 1 followed by Delete at index 1.
        if instructions.is_empty() && signal_instructions.is_empty() {
            self.instructions.clear();
            return Ok(());
        }

        // Depending on the delete strategy, update EDF+ files to be discontinuous after deleting a record
        let removes_middle_record = instructions.iter().any(|i| matches!(i, SaveInstruction::Remove(idx) if *idx > 0 && *idx < self.record_counter - 1));
        if self.header.specification == EDFSpecifications::EDFPlus
            && self.header.is_continuous
            && removes_middle_record
            && self.record_delete_strategy == RecordDeleteStrategy::Discontinuous
        {
            self.header.is_continuous = false;
        }

        let patch_trailing_records = !signal_instructions.is_empty();
        let mut overwrite_counter = header_size_diff;
        let mut overwrite_buffer = Vec::new();
        let mut record_counter = instructions
            .iter()
            .filter(|i| i.has_record_index())
            .map(SaveInstruction::record_index)
            .next()
            .unwrap_or(0);
        let mut instruction_idx = 0;

        if patch_trailing_records {
            record_counter = 0;
        }

        // Seek to first data-record edit position. In case the file header has to be written, this seek operation will be useless
        file.seek(SeekFrom::Start(
            initial_header_size + record_counter as u64 * initial_record_bytes as u64,
        ))
        .map_err(EDFError::FileWriteError)?;

        // Loop through all instructions and perform each of them
        loop {
            let instruct = match instructions.get(instruction_idx) {
                Some(instruct) => instruct,
                None => {
                    if patch_trailing_records {
                        &SaveInstruction::Patch
                    } else {
                        break;
                    }
                }
            };

            match instruct {
                SaveInstruction::WriteHeader => {
                    instruction_idx += 1;

                    // NOTE: This instruction must not be called more than once per save operation and
                    // must be called as the first instruction if it is present!

                    file.seek(SeekFrom::Start(0))
                        .map_err(EDFError::FileWriteError)?;
                    if let Ok(read_length) = usize::try_from(overwrite_counter)
                        && read_length > 0
                    {
                        let read_max = initial_filesize.saturating_sub(initial_header_size);
                        let read_length = read_length.min(read_max as usize);
                        if read_length > 0 {
                            let mut buffer = vec![0; read_length]; // TODO: This should be a function global defined buffer
                            file.read_exact_at(&mut buffer, initial_header_size)
                                .map_err(EDFError::FileWriteError)?;
                            overwrite_buffer.append(&mut buffer);
                        }
                    }

                    file.write_all(self.header.serialize()?.as_bytes())
                        .map_err(EDFError::FileWriteError)?;

                    // Require re-writing all data-records from the beginning due to a change in offset
                    if overwrite_counter != 0 {
                        record_counter = 0;
                    } else if !patch_trailing_records {
                        file.seek(SeekFrom::Start(
                            initial_header_size
                                + record_counter as u64 * initial_record_bytes as u64,
                        ))
                        .map_err(EDFError::FileWriteError)?;
                    }
                }
                SaveInstruction::Remove(idx) if *idx == record_counter => {
                    instruction_idx += 1;
                    _ = overwrite_buffer
                        .drain(0..initial_record_bytes.min(overwrite_buffer.len()))
                        .count();
                    overwrite_counter -= initial_record_bytes as i64;
                }
                SaveInstruction::Insert(idx, SaveValue::Record(value))
                    if *idx == record_counter =>
                {
                    instruction_idx += 1;
                    record_counter += 1;

                    // println!("I - BEFORE {} / {} / {}", overwrite_counter, overwrite_buffer.len(), file.stream_position().map_err(SerializeError::FileWriteError)?.saturating_sub(1024));
                    let read_offset = if overwrite_counter < 0 {
                        overwrite_counter.abs()
                    } else {
                        0
                    } as u64;
                    let current_file_position =
                        file.stream_position().map_err(EDFError::FileWriteError)? + read_offset;
                    let read_max = initial_filesize.saturating_sub(current_file_position);
                    if let Ok(new_buffer_length) =
                        usize::try_from(overwrite_counter + new_record_bytes as i64)
                        && new_buffer_length > 0
                    {
                        let read_length = new_buffer_length - overwrite_buffer.len(); // This will only ever be different from `new_record_bytes` in case `overwrite_counter` was negative and < `new_record_bytes`
                        let read_length = read_length.min(read_max as usize);
                        if read_length > 0 {
                            let mut buffer = vec![0; read_length]; // TODO: This should be a function global defined buffer
                            file.read_exact_at(&mut buffer, current_file_position)
                                .map_err(EDFError::FileWriteError)?;
                            overwrite_buffer.append(&mut buffer);
                        }
                    }

                    file.write_all(&value.serialize()?)
                        .map_err(EDFError::FileWriteError)?;
                    overwrite_counter += new_record_bytes.min(read_max as usize) as i64;
                }
                SaveInstruction::Update(idx, SaveValue::Record(value))
                    if *idx == record_counter =>
                {
                    instruction_idx += 1;
                    record_counter += 1;

                    let buffer_read_count = overwrite_buffer
                        .drain(0..initial_record_bytes.min(overwrite_buffer.len()))
                        .count();
                    let disk_read_count = initial_record_bytes.saturating_sub(buffer_read_count);

                    let read_offset = if overwrite_counter < 0 {
                        overwrite_counter.abs()
                    } else {
                        0
                    } as u64;
                    let buffered_offset = if overwrite_counter > 0 {
                        overwrite_counter
                    } else {
                        0
                    } as u64;
                    let current_file_position =
                        file.stream_position().map_err(EDFError::FileWriteError)? + read_offset;

                    // Add data to overwrite buffer which would be overwritten after writing the current record
                    let read_max = initial_filesize.saturating_sub(current_file_position);
                    let target_read_length =
                        overwrite_counter + new_record_bytes as i64 - buffered_offset as i64;
                    let read_length = u64::try_from(target_read_length)
                        .map(|len| len.min(read_max).saturating_sub(disk_read_count as u64))
                        .unwrap_or(0) as usize;
                    if read_length > 0 {
                        let mut buffer = vec![0; read_length]; // TODO: This should be a function global defined buffer
                        file.read_exact_at(
                            &mut buffer,
                            current_file_position + disk_read_count as u64,
                        )
                        .map_err(EDFError::FileWriteError)?;
                        overwrite_buffer.append(&mut buffer);
                    }

                    // When coming to end of file, instead of adding the diff, remove everything that would read past the file end
                    let exceed = if target_read_length.max(0) as u64 > read_max {
                        target_read_length.max(0) as u64 - read_max
                    } else {
                        0
                    };
                    overwrite_counter += new_record_bytes as i64
                        - disk_read_count as i64
                        - buffer_read_count as i64
                        - exceed as i64;

                    file.write_all(&value.serialize()?)
                        .map_err(EDFError::FileWriteError)?;
                }
                SaveInstruction::Patch | _ => {
                    // Break if the last available record has already been written
                    if record_counter == self.record_counter {
                        break;
                    }

                    // In case the file offset is at the beginning of the next data-record, seek to the next position
                    // in the file where another instruction has to be handled. All records between the current
                    // position and the target position remain entirely unchanged (as long as signals have not changed)
                    if overwrite_counter == 0 && !patch_trailing_records {
                        record_counter = instructions
                            .iter()
                            .skip(instruction_idx)
                            .find_map(|i| {
                                if i.record_index() != usize::MAX {
                                    Some(i.record_index())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0);
                        file.seek(SeekFrom::Start(
                            initial_header_size
                                + record_counter as u64 * initial_record_bytes as u64,
                        ))
                        .map_err(EDFError::FileWriteError)?;
                        continue;
                    }

                    // Try to read the record from the overwrite buffer
                    let read_offset = if overwrite_counter < 0 {
                        overwrite_counter.abs()
                    } else {
                        0
                    } as u64;
                    let buffered_offset = if overwrite_counter > 0 {
                        overwrite_counter
                    } else {
                        0
                    } as u64;
                    let mut buffer_read = overwrite_buffer
                        .drain(0..initial_record_bytes.min(overwrite_buffer.len()))
                        .collect::<Vec<_>>();
                    let buffer_read_count = buffer_read.len();
                    let disk_read_count = initial_record_bytes - buffer_read_count;

                    // Read the remaining bytes of the current record from disk if it was not entirely in the buffer
                    let current_file_position =
                        file.stream_position().map_err(EDFError::FileWriteError)? + read_offset;
                    if disk_read_count > 0 {
                        let mut buffer = vec![0; disk_read_count]; // TODO: This should be a function global defined buffer
                        file.read_exact_at(&mut buffer, current_file_position)
                            .map_err(EDFError::FileWriteError)?;
                        buffer_read.append(&mut buffer);
                    }

                    // Add data to overwrite buffer which would be overwritten after writing the current record
                    let read_max = initial_filesize.saturating_sub(current_file_position);
                    let target_read_length =
                        overwrite_counter + new_record_bytes as i64 - buffered_offset as i64;
                    let read_length = u64::try_from(target_read_length)
                        .map(|len| len.min(read_max).saturating_sub(disk_read_count as u64))
                        .unwrap_or(0) as usize;
                    if read_length > 0 {
                        let mut buffer = vec![0; read_length]; // TODO: This should be a function global defined buffer
                        file.read_exact_at(
                            &mut buffer,
                            current_file_position + disk_read_count as u64,
                        )
                        .map_err(EDFError::FileWriteError)?;
                        overwrite_buffer.append(&mut buffer);
                    }

                    // When coming to end of file, instead of adding the diff, remove everything that would read past the file end
                    let exceed = if target_read_length.max(0) as u64 > read_max {
                        target_read_length.max(0) as u64 - read_max
                    } else {
                        0
                    };
                    overwrite_counter += new_record_bytes as i64
                        - disk_read_count as i64
                        - buffer_read_count as i64
                        - exceed as i64;

                    // In case the signals changed, patch the record and update the buffer with the patched data
                    if !signal_instructions.is_empty() {
                        let cursor = Cursor::new(buffer_read);
                        let mut reader = BufReader::new(cursor);
                        let mut record = Self::read_record_data(
                            &mut reader,
                            0,
                            &initial_signals,
                            initial_record_duration,
                        )?;
                        record.patch_record(&signal_instructions)?;
                        buffer_read = record.serialize()?;
                    }

                    file.write_all(&buffer_read)
                        .map_err(EDFError::FileWriteError)?;
                    record_counter += 1;
                }
            }
        }

        // The file size has changed, therefore either write the remaining buffered data to disk, or
        // truncate the file (and replacing the content to be truncated with NUL bytes before)
        if overwrite_counter != 0 {
            if overwrite_counter > 0 {
                file.write_all(&overwrite_buffer)
                    .map_err(EDFError::FileWriteError)?;
                overwrite_buffer.clear();
            } else {
                let reduced_by_length = overwrite_counter.abs() as usize;
                let position = file.stream_position().map_err(EDFError::FileWriteError)?;
                file.write_all(&repeat_n(0, reduced_by_length).collect::<Vec<_>>())
                    .map_err(EDFError::FileWriteError)?;
                file.set_len(position).map_err(EDFError::FileWriteError)?;
            }
        }

        // Flush the write buffer, clear the pending instructions and get the new file length
        file.flush().map_err(EDFError::FileWriteError)?;
        self.instructions.clear();
        let new_file_size = file.metadata().map_err(EDFError::FileWriteError)?.len();

        // Update the initial record size and header hash so they are valid for the current state.
        // This ensures the next save action works with the right offsets and instructions
        self.header.update_initial_record_bytes();
        self.header.update_initial_header_sha256()?;

        // Try to seek to the position the reader initially was at
        if let Some(record_idx) = initial_record_position {
            let seek_pos = self.header.header_bytes as u64 + record_idx * new_record_bytes as u64;
            self.reader
                .seek(SeekFrom::Start(seek_pos.min(new_file_size)))
                .map_err(EDFError::FileWriteError)?;
        } else {
            self.reader
                .seek(SeekFrom::Start(0))
                .map_err(EDFError::FileWriteError)?;
        }

        Ok(())
    }

    pub fn read_record(&mut self) -> Result<Option<Record>, EDFError> {
        // TODO: Try to read the record from a state after save in case it was not yet saved. Meaning e.g.
        // records A, B, C, D are stored in the EDF and then E was inserted at index 2, the records returned
        // by reading all records from the front should result in A, B, E, C, D before and after saving. Therefore
        // the records would have to be able to read either from in-memory pending instructions or from disk (at the
        // correct offset). Also handle add/remove of signals to those in-memory records as well

        let position = self
            .reader
            .stream_position()
            .map_err(EDFError::FileReadError)?;

        // Ensure the reader position is in the data-record section of the file
        if position < self.header.header_bytes as u64 {
            return Err(EDFError::InvalidReadRange);
        }

        // Ensure the reader position is at the beginning of a data-record
        let record_size = self.header.data_record_bytes() as u64;
        let record_offset = position - self.header.header_bytes as u64;
        if record_offset % record_size != 0 {
            return Err(EDFError::InvalidReadRange);
        }

        // Get the data-record index and check if there are any records left in the file
        let record_idx = (position - self.header.header_bytes as u64) / record_size;
        let record_count = self
            .header
            .record_count
            .ok_or(EDFError::ReadWhileRecording)?;
        if record_idx + 1 > record_count as u64 {
            return Ok(None);
        }

        // Read and parse the record from disk
        let mut record = Self::read_record_data(
            &mut self.reader,
            record_idx,
            &self.header.signals,
            self.header.record_duration,
        )?;

        // Patch the record to match the new signal definitions
        record.patch_record(&self.instructions)?;

        Ok(Some(record))
    }

    fn read_record_data<R: BufRead + Seek>(
        reader: &mut R,
        record_idx: u64,
        signals: &Vec<SignalHeader>,
        record_duration: f64,
    ) -> Result<Record, EDFError> {
        let mut sample_buffer = [0; 2];
        let mut tal_buffer = vec![];
        let mut record = Record::new(&signals);
        record.default_offset = record_idx as f64 * record_duration;

        for (i, signal) in signals.iter().enumerate() {
            if signal.is_annotation() {
                // Samples are 16 bit integers (1 sample has 2 bytes) therefore annotation samples are * 2
                // as only single byte values are being read
                let mut tals = Vec::new();
                let mut total_read = 0;
                while total_read < signal.samples_count * 2 {
                    total_read += reader
                        .read_until(b'\x00', &mut tal_buffer)
                        .map_err(EDFError::FileReadError)?;

                    // Check if EOF has been reached
                    if tal_buffer.is_empty() {
                        break;
                    }

                    // Check if the read value is a NUL byte, meaning it most likely reached the
                    // padding of the TAL in the current data-record. This would mean it should probably
                    // seek to the end of the data-record instead of reading every byte individually. There
                    // should not be any other TAL following then
                    if tal_buffer.len() == 1 && tal_buffer[0] == b'\x00' {
                        tal_buffer.clear();
                        continue;
                    }

                    // Parse the TAL and add it to the list of TALs in the current signal
                    let tal = AnnotationList::deserialize(&take_vec(&mut tal_buffer))?;
                    tals.push(tal);
                }
                record.set_annotation(i, tals)?;
            } else {
                let mut samples = Vec::with_capacity(signal.samples_count);
                for _ in 0..signal.samples_count {
                    reader
                        .read_exact(&mut sample_buffer)
                        .map_err(EDFError::FileReadError)?;
                    let sample = i16::from_le_bytes(sample_buffer);
                    samples.push(sample);
                }
                record.set_samples(i, samples)?;
            }
        }

        Ok(record)
    }

    pub fn read_record_at(&mut self, index: usize) -> Result<Option<Record>, EDFError> {
        self.seek_to_record(index)?;
        self.read_record()
    }

    pub fn seek_to_record(&mut self, index: usize) -> Result<(), EDFError> {
        self.reader
            .seek(SeekFrom::Start(
                self.header.header_bytes as u64
                    + index as u64 * self.header.data_record_bytes() as u64,
            ))
            .map_err(EDFError::FileReadError)?;
        Ok(())
    }

    pub fn seek_previous_record(&mut self) -> Result<bool, EDFError> {
        // Check if the current reader position is already at or before the first data-record.
        // In that case, this function will not do anything and return false.
        let position = self
            .reader
            .stream_position()
            .map_err(EDFError::FileReadError)?;
        if position <= self.header.header_bytes as u64 {
            return Ok(false);
        }

        self.reader
            .seek(SeekFrom::Current(-(self.header.data_record_bytes() as i64)))
            .map_err(EDFError::FileReadError)?;

        Ok(true)
    }

    /// Reads samples and annotations for the given duration starting at the current reader position.
    /// Regular EDF files and continuous EDF+ files will return a Vec with exactly 1 entry in each signal
    /// in the `signal_samples` array when any data-records could be read. Discontinuous EDF+ files though can
    /// return a Vec of any size. It will be of length 0 when the read duration is entirely between two records.
    /// An additional item in the samples vec indicates a gap between the 2 data-records. This means e.g. the length will be 2
    /// if you were to read 90 seconds and the first data-record is at offset 0 and the second data-record is at offset
    /// 60 and the data-record duration is 30 seconds. Therefore there would be a gap of 30 seconds
    /// between both of the data-records.
    ///
    /// Note: In case of EDF+ files the list of annotations returned will contain all
    /// `Time-keeping Timestamped-Annotation-List` entries. Therefore if you were to read across 5 data-records,
    /// you will get at least 5 Time-keeping TALs returned in the `annotations` of the `SpanningRecord`
    pub fn read_nanos(&mut self, nanoseconds: u128) -> Result<SpanningRecord, EDFError> {
        let offset_end = self.record_read_offset_ns + nanoseconds;
        let record_duration_ns = (self.header.record_duration * 1_000_000_000.0) as u128;

        // Note: In case of an error while reading a record, the buffer reader
        // is not being reset to the original position. This means e.g. when trying to
        // read 6 records, the reader might have read 2 records, failed at the 3rd one
        // and did not read the rest. When calling `read_record` after this now (if it were
        // to succeed then), it would return the 3rd record.
        let mut records = SpanningRecord::new(&self.header);
        let mut offset_current = self.record_read_offset_ns;
        let mut read_start_ns = if self.seek_previous_record()? {
            self.read_record()?
                .map(|r| (r.get_start_offset() * 1_000_000_000.0) as u128)
        } else {
            None
        };
        let mut remaining_record_ns = 0;

        // Read until either reaching the desired read duration or until no more records are available
        while offset_current < offset_end {
            let Some(mut record) = self.read_record()? else {
                remaining_record_ns = 0;
                break;
            };

            // Get the amount of nano seconds to in between the previous record and the current
            let onset = (record.get_start_offset() * 1_000_000_000.0) as u128;
            let skip_duration_ns = if self.header.specification == EDFSpecifications::EDF {
                0
            } else if let Some(previous_onset) = &read_start_ns {
                onset - previous_onset - record_duration_ns
            } else {
                let already_skipped = onset - offset_current;
                onset - already_skipped
            };

            if read_start_ns.is_none() {
                read_start_ns = Some(onset);
            }

            let sample_frequencies = record
                .signal_samples
                .iter()
                .map(|s| s.len() as f64 / self.header.record_duration)
                .collect::<Vec<_>>();

            // Get the read offset in nano seconds within the current record and remove
            // signal samples and annotations which occur before / do not last until the offset
            let record_offset_ns = if offset_current == self.record_read_offset_ns {
                records.insert_spanning_wait(
                    record.get_start_offset() + self.record_read_offset_ns as f64 / 1_000_000_000.0,
                );

                // Drop samples and annotations before and not lasting until the current offset
                if self.record_read_offset_ns > 0 {
                    // Remove all signal samples which are before the current read offset
                    for signal in record.signal_samples.iter_mut() {
                        let sample_freq = signal.len() as f64 / self.header.record_duration;
                        let sample_count = (self.record_read_offset_ns as f64 / 1_000_000_000.0
                            * sample_freq)
                            .floor() as usize;
                        signal.drain(..sample_count);
                    }

                    // Remove all annotations which are not record global (duration of 0) and are not
                    // within the record read start time frame
                    record.annotations.iter_mut().for_each(|a| {
                        a.retain(|annotation| {
                            if annotation.duration == 0.0 {
                                return true;
                            }
                            let annotation_onset_ns = (annotation.onset * 1_000_000_000.0) as u128;
                            let annotation_duration_ns =
                                (annotation.duration * 1_000_000_000.0) as u128;
                            return annotation_onset_ns + annotation_duration_ns
                                >= read_start_ns.unwrap() + self.record_read_offset_ns;
                        })
                    });
                }

                self.record_read_offset_ns
            } else {
                0
            };

            // If there is a time gap in between records, start a new spanning entry
            if skip_duration_ns != 0 && !records.is_spanning_wait() {
                records.insert_spanning_wait(
                    record.get_start_offset() + record_offset_ns as f64 / 1_000_000_000.0,
                );
            }

            // Add the skip duration and break if the read duration has been reached (meaning the
            // rest of the data to read was within the time gap)
            offset_current += skip_duration_ns;
            if offset_current >= offset_end {
                self.seek_previous_record()?;
                break;
            }

            // Take the desired amount of samples from the current record
            remaining_record_ns = offset_end - offset_current;
            let record_remaining_ns = record_duration_ns - self.record_read_offset_ns;
            if remaining_record_ns >= record_remaining_ns {
                for (i, signal) in record.signal_samples.iter().enumerate() {
                    records.extend_samples(i, signal.to_vec())
                }

                records.annotations.extend(record.annotations);
                offset_current += record_remaining_ns;
                remaining_record_ns -= record_remaining_ns;

                // Reading the record has finished and therefore the next record should be read from the start again
                self.record_read_offset_ns = 0;
            } else {
                for (i, signal) in record.signal_samples.iter().enumerate() {
                    let sample_freq = sample_frequencies[i];
                    let prev_sample_count =
                        self.record_read_offset_ns as f64 / 1_000_000_000.0 * sample_freq;
                    let current_sample_count =
                        remaining_record_ns as f64 / 1_000_000_000.0 * sample_freq;
                    let total_sample_count = prev_sample_count + current_sample_count;
                    let sample_count =
                        (total_sample_count - prev_sample_count.floor()).floor() as usize;
                    records.extend_samples(i, signal[..sample_count].to_vec())
                }

                // Add all annotations which start before the end of the desired read duration
                for tal_list in record.annotations {
                    let mut tals = Vec::new();
                    for annotation_list in tal_list {
                        let annotation_onset_ns = (annotation_list.onset * 1_000_000_000.0) as u128;
                        let is_entire_record = annotation_list.duration == 0.0;
                        let is_starting_until_read_end =
                            annotation_onset_ns <= read_start_ns.unwrap() + offset_end;
                        if is_entire_record || is_starting_until_read_end {
                            tals.push(annotation_list);
                        }
                    }
                    records.annotations.push(tals);
                }

                self.seek_previous_record()?;
                break;
            }

            read_start_ns = Some(onset);
        }

        // Finish the record (to remove any potentially trailing empty spans)
        records.finish();

        // Update the current record offset after reading
        self.record_read_offset_ns += remaining_record_ns;

        Ok(records)
    }

    pub fn read_micros(&mut self, microseconds: u128) -> Result<SpanningRecord, EDFError> {
        self.read_nanos(microseconds * 1_000)
    }

    pub fn read_millis(&mut self, milliseconds: u128) -> Result<SpanningRecord, EDFError> {
        self.read_nanos(milliseconds * 1_000_000)
    }

    pub fn read_seconds(&mut self, seconds: u128) -> Result<SpanningRecord, EDFError> {
        self.read_nanos(seconds * 1_000_000_000)
    }

    /// Reads samples and annotations for the given duration starting at the current reader position.
    /// Regular EDF files and continuous EDF+ files will return a Vec with exactly 1 entry in each signal
    /// in the `signal_samples` array when any data-records could be read. Discontinuous EDF+ files though can
    /// return a Vec of any size. It will be of length 0 when the read duration is entirely between two records.
    /// An additional item in the samples vec indicates a gap between the 2 data-records. This means e.g. the length will be 2
    /// if you were to read 90 seconds and the first data-record is at offset 0 and the second data-record is at offset
    /// 60 and the data-record duration is 30 seconds. Therefore there would be a gap of 30 seconds
    /// between both of the data-records.
    ///
    /// Note: In case of EDF+ files the list of annotations returned will contain all
    /// `Time-keeping Timestamped-Annotation-List` entries. Therefore if you were to read across 5 data-records,
    /// you will get at least 5 Time-keeping TALs returned in the `annotations` of the `SpanningRecord`
    ///
    /// Note: This function converts seconds to nano seconds internally. This will result in slightly inaccurate
    /// results due to floating point imprecision. For more exact values, use the `read_nanos(...)` or `read_millis(...)` functions
    pub fn read_seconds_approx(&mut self, seconds: f32) -> Result<SpanningRecord, EDFError> {
        if seconds <= 0.0 {
            return Err(EDFError::InvalidReadRange);
        }

        self.read_nanos((seconds as f64 * 1_000_000_000.0) as u128)
    }
}
