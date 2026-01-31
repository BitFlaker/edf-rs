use crate::headers::signal_header::SignalHeader;
use crate::record::Record;

pub fn normalize_instructions(
    instructions: &Vec<SaveInstruction>,
    initial_count: usize,
) -> Vec<SaveInstruction> {
    let mut normalized_instructions = Vec::new();

    // Counter keeping track of the amount of elements after the current instruction.
    // This is only required to be able to translate APPEND instructions into an INSERT
    // instruction with the given index
    let mut item_counter = initial_count;

    // Go through every instruction and normalize it and add it to the final list of
    // normalized instructions and remove/ignore instructions cancelling out each other
    for tr in instructions {
        // Turn an APPEND instruction into an INSERT instruction
        let mut instruction = if let SaveInstruction::Append(c) = tr {
            SaveInstruction::Insert(item_counter, c.clone())
        } else {
            tr.clone()
        };

        // Adjust previous instructions to new states
        let mut add_instruction = true;
        match instruction {
            SaveInstruction::Insert(current_idx, _) => {
                for current in normalized_instructions.iter_mut().rev() {
                    match current {
                        SaveInstruction::Insert(idx, _) | SaveInstruction::Update(idx, _)
                            if *idx >= current_idx =>
                        {
                            *idx += 1
                        }

                        // Delete needs to be larger than and not just equal as otherwise
                        // the instruction would immediately delete the inserted value
                        SaveInstruction::Remove(idx) if *idx > current_idx => *idx += 1,

                        // Any other unaffected operations
                        _ => {}
                    };
                }

                item_counter += 1;
            }
            SaveInstruction::Update(current_idx, ref c) => {
                // If there was an INSERT instruction before which would be updated by the current UPDATE instruction,
                // replace the initial INSERT and the current UPDATE instructions with a single INSERT instruction.
                if let Some(idx) = normalized_instructions.iter().position(
                    |tr| matches!(tr, SaveInstruction::Insert(idx, _) if *idx == current_idx),
                ) {
                    normalized_instructions.remove(idx);
                    instruction = SaveInstruction::Insert(current_idx, c.clone());
                }
                // Replace a previous UPDATE instruction which would be replaced by the current UPDATE instruction with the current one
                else if let Some(idx) = normalized_instructions.iter().position(
                    |tr| matches!(tr, SaveInstruction::Update(idx, _) if *idx == current_idx),
                ) {
                    normalized_instructions.remove(idx);
                };
            }
            SaveInstruction::Remove(current_idx) => {
                let mut idx_eliminate = -1;
                for (i, current) in normalized_instructions.iter_mut().enumerate().rev() {
                    match current {
                        // Any instructions with index after delete
                        SaveInstruction::Insert(idx, _)
                        | SaveInstruction::Update(idx, _)
                        | SaveInstruction::Remove(idx)
                            if *idx > current_idx =>
                        {
                            *idx -= 1
                        }

                        // Insert and delete cancel each other out
                        SaveInstruction::Insert(idx, _)
                            if *idx == current_idx && idx_eliminate == -1 =>
                        {
                            idx_eliminate = i as i64;
                            add_instruction = false;
                        }

                        // Update cancelled out by delete
                        SaveInstruction::Update(idx, _)
                            if *idx == current_idx && idx_eliminate == -1 =>
                        {
                            idx_eliminate = i as i64
                        }

                        // Any other unaffected operations
                        _ => {}
                    };
                }

                // If any, remove the instruction which will become useless due to this delete instruction.
                // E.g. When there is an INSERT at index 3 and this is a DELETE at index 3 instruction. Therefore
                // both instructions cancel each other out. An UPDATE followed by a DELETE would cause the UPDATE to
                // have no effect, as the DELETE would remove the item at that index anyways
                if idx_eliminate >= 0 {
                    normalized_instructions.remove(idx_eliminate as usize);
                }

                item_counter -= 1;
            }
            SaveInstruction::WriteHeader => {}

            // Convenience operations which are not supposed to be in the final instruction list
            // should not be handled (e.g. APPEND will always turn into INSERT and therefore does not
            // require any handling)
            _ => {
                add_instruction = false;
            }
        };

        // Add the instruction to the final list in case it did not get cancelled out with another operation (
        // this would only be the case when an INSERT instruction was deleted again)
        if add_instruction {
            normalized_instructions.push(instruction);
        }
    }

    // Sort instructions by index and by their priority (equal indices sort by their instruction type in the order of [DELETE; INSERT; UPDATE])
    // to make all indices valid and to be able to work into a single direction
    normalized_instructions.sort_by(|a, b| {
        a.index()
            .cmp(&b.index())
            .then_with(|| a.priority().cmp(&b.priority()))
    });

    // Merge DELETE instructions followed by an INSERT instruction with both having the same index into a single UPDATE instruction
    merge_to_updates(normalized_instructions)
}

/// Merges a delete instruction immediately followed by an insert instruction where both are targeting
/// the same index into a single Update instruction.
fn merge_to_updates(instructions: Vec<SaveInstruction>) -> Vec<SaveInstruction> {
    let mut out = Vec::with_capacity(instructions.len());
    let mut iter = instructions.into_iter().peekable();

    while let Some(curr) = iter.next() {
        match (&curr, iter.peek()) {
            (SaveInstruction::Remove(idx_d), Some(SaveInstruction::Insert(idx_i, c)))
                if idx_d == idx_i =>
            {
                out.push(SaveInstruction::Update(*idx_d, c.clone()));
                iter.next();
            }
            _ => out.push(curr),
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaveValue {
    Record(Record),
    Signal(SignalHeader),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaveInstruction {
    WriteHeader,
    Update(usize, SaveValue),
    Insert(usize, SaveValue),
    Append(SaveValue),
    Remove(usize),
    Patch,
}

impl SaveInstruction {
    pub fn index(&self) -> usize {
        match self {
            SaveInstruction::WriteHeader => 0,
            SaveInstruction::Remove(idx)
            | SaveInstruction::Insert(idx, _)
            | SaveInstruction::Update(idx, _) => *idx,
            _ => usize::MAX,
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            SaveInstruction::WriteHeader => 0,
            SaveInstruction::Remove(_) => 1,
            SaveInstruction::Insert(_, _) => 2,
            SaveInstruction::Update(_, _) => 3,
            _ => u8::MAX,
        }
    }

    pub fn has_record_index(&self) -> bool {
        match self {
            SaveInstruction::Remove(_)
            | SaveInstruction::Insert(_, _)
            | SaveInstruction::Update(_, _) => true,
            _ => false,
        }
    }

    pub fn record_index(&self) -> usize {
        if !self.has_record_index() {
            return usize::MAX;
        }

        match self {
            SaveInstruction::Remove(idx)
            | SaveInstruction::Insert(idx, _)
            | SaveInstruction::Update(idx, _) => *idx,
            _ => usize::MAX,
        }
    }
}
