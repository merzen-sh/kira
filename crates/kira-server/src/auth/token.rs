//! Session token encoding helpers.

/// Version prefix for signed session tokens.
pub(super) const TOKEN_VERSION: &str = "v1";

pub(super) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

pub(super) fn from_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }

    let (pairs, _) = value.as_bytes().as_chunks::<2>();
    let mut bytes = Vec::with_capacity(pairs.len());
    for [high_byte, low_byte] in pairs {
        let high = hex_digit(*high_byte)?;
        let low = hex_digit(*low_byte)?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(from_hex(&to_hex(&bytes)), Some(bytes.to_vec()));
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
    }
}
