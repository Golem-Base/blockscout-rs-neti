use bytes::Bytes;

pub fn bytes_to_hex(bytes: &Bytes) -> String {
    let slice = bytes.as_ref();
    let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
    format!("0x{hex}")
}

pub fn iso_to_ts_sec(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .unwrap()
        .timestamp()
        .to_string()
}
