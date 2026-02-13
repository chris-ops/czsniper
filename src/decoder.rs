use alloy::primitives::U256;
use anyhow::{Result, anyhow};

pub fn decode_custom_log(data: &[u8]) -> Result<(String, String)> {
    if data.len() < 32 * 8 {
        return Err(anyhow!("Data too short for header"));
    }

    // word0: address A (skip 12 bytes padding)
    // word1: address B (skip 12 bytes padding)
    // word2: uint256
    // word3: offset A
    // word4: offset B
    // word5-7: uint256

    let offset_a = parse_uint256(&data[32*3..32*4])?.to::<usize>();
    let offset_b = parse_uint256(&data[32*4..32*5])?.to::<usize>();

    let string_a = extract_string(data, offset_a)?;
    let string_b = extract_string(data, offset_b)?;

    Ok((string_a, string_b))
}

fn parse_uint256(chunk: &[u8]) -> Result<U256> {
    if chunk.len() != 32 {
        return Err(anyhow!("Invalid chunk size for uint256"));
    }
    Ok(U256::from_be_slice(chunk))
}

fn extract_string(data: &[u8], offset: usize) -> Result<String> {
    if offset + 32 > data.len() {
        return Err(anyhow!("Offset out of bounds for string length at {}", offset));
    }

    let length = parse_uint256(&data[offset..offset+32])?.to::<usize>();
    let start = offset + 32;
    let end = start + length;

    if end > data.len() {
        return Err(anyhow!("String data out of bounds: offset={}, length={}, data_len={}", offset, length, data.len()));
    }

    let string_bytes = &data[start..end];
    Ok(String::from_utf8_lossy(string_bytes).into_owned())
}

pub fn contains_chinese(s: &str) -> bool {
    s.chars().any(|c| {
        // Range for common Chinese characters (Unified Ideographs)
        (c >= '\u{4E00}' && c <= '\u{9FFF}') ||
        // Extended ranges if necessary
        (c >= '\u{3400}' && c <= '\u{4DBF}') ||
        (c >= '\u{20000}' && c <= '\u{2A6DF}')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_chinese() {
        assert!(contains_chinese("Hello 世界"));
        assert!(contains_chinese("币安"));
        assert!(!contains_chinese("Hello World"));
        assert!(!contains_chinese("1234567890!@#$%"));
    }
}
