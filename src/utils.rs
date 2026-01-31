pub(crate) fn take_vec<T>(vec: &mut Vec<T>) -> Vec<T> {
    std::mem::take(vec)
}

pub(crate) fn serialize_field(value: Option<String>) -> String {
    value.map(|v| v.replace(" ", "_")).unwrap_or("X".to_string())
}

pub(crate) fn deserialize_field(value: &str) -> Option<String> {
    if value == "X" {
        return None;
    }

    // TODO: Add serializer option to replace `_` with ` ` automatically
    Some(value.replace("_", " "))
}

pub(crate) fn is_printable_ascii(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, 0x20..=0x7E))
}
