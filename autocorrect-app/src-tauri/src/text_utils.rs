//! Text-processing utilities for byte/char/UTF-16 offset conversions
//! and visual-width estimation.

pub fn byte_offset_to_utf16_offset(text: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(text.len());
    text[..clamped].encode_utf16().count()
}

/// Convert multiple byte offsets to UTF-16 offsets in a single O(text_len) scan.
/// `offsets` must produce values in **ascending** order (guaranteed by caller).
/// Returns a Vec in the same order as the input offsets.
pub fn batch_byte_to_utf16_offsets(
    text: &str,
    offsets: impl Iterator<Item = usize>,
) -> Vec<usize> {
    let offsets: Vec<usize> = offsets.collect();
    let mut result = vec![0usize; offsets.len()];
    if offsets.is_empty() {
        return result;
    }
    let mut u16_count = 0usize;
    let mut byte_pos = 0usize;
    let mut idx = 0usize;
    for ch in text.chars() {
        // Fill all offsets that land at or before the current byte position
        while idx < offsets.len() && offsets[idx] <= byte_pos {
            result[idx] = u16_count;
            idx += 1;
        }
        if idx >= offsets.len() {
            break;
        }
        u16_count += ch.len_utf16();
        byte_pos += ch.len_utf8();
    }
    // Fill any remaining offsets beyond end-of-text
    while idx < offsets.len() {
        result[idx] = u16_count;
        idx += 1;
    }
    result
}

pub fn byte_offset_to_char_offset(text: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(text.len());
    text[..clamped].chars().count()
}

pub fn utf16_offset_to_char_offset(text: &str, utf16_offset: usize) -> usize {
    let mut chars = 0usize;
    let mut seen_u16 = 0usize;
    for ch in text.chars() {
        if seen_u16 >= utf16_offset {
            break;
        }
        seen_u16 += ch.len_utf16();
        chars += 1;
    }
    chars
}

pub fn char_offset_to_line_col(text: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in text.chars().enumerate() {
        if idx >= char_offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

pub fn estimate_word_visual_width(word: &str, avg_char_width: f64) -> f64 {
    let units = visual_units(word);
    (units * avg_char_width * 0.9).max(avg_char_width * 0.7)
}

pub fn visual_width_for_prefix(text: &str, char_count: usize, avg_char_width: f64) -> f64 {
    visual_units_for_prefix(text, char_count) * avg_char_width * 0.9
}

pub fn char_index_to_byte_offset(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

pub fn visual_units(text: &str) -> f64 {
    text.chars()
        .fold(0.0, |acc, ch| acc + glyph_width_factor(ch))
}

pub fn visual_units_for_prefix(text: &str, char_count: usize) -> f64 {
    text.chars()
        .take(char_count)
        .fold(0.0, |acc, ch| acc + glyph_width_factor(ch))
}

pub fn glyph_width_factor(ch: char) -> f64 {
    match ch {
        'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' => 0.55,
        'm' | 'w' | 'M' | 'W' | '@' | '%' | '&' | 'Q' | 'O' => 1.3,
        'A'..='Z' => 1.05,
        '0'..='9' => 0.9,
        _ if ch.is_ascii_punctuation() => 0.5,
        _ => 0.92,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_utf16_offset_ascii() {
        assert_eq!(byte_offset_to_utf16_offset("hello", 0), 0);
        assert_eq!(byte_offset_to_utf16_offset("hello", 2), 2);
        assert_eq!(byte_offset_to_utf16_offset("hello", 5), 5);
    }

    #[test]
    fn test_byte_offset_to_utf16_offset_cjk() {
        let text = "你好hello";
        // 你 = 3 bytes, 1 UTF-16 code unit
        assert_eq!(byte_offset_to_utf16_offset(text, 0), 0);
        assert_eq!(byte_offset_to_utf16_offset(text, 3), 1);
        assert_eq!(byte_offset_to_utf16_offset(text, 6), 2);
        assert_eq!(byte_offset_to_utf16_offset(text, 7), 3);
    }

    #[test]
    fn test_byte_offset_to_utf16_offset_emoji() {
        let text = "a😀b";
        // 😀 = 4 bytes, 2 UTF-16 code units
        assert_eq!(byte_offset_to_utf16_offset(text, 1), 1);
        assert_eq!(byte_offset_to_utf16_offset(text, 5), 3);
    }

    #[test]
    fn test_batch_byte_to_utf16_offsets() {
        let text = "你好hello";
        let offsets = [0, 3, 6, 7];
        let result = batch_byte_to_utf16_offsets(text, offsets.into_iter());
        assert_eq!(result, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_batch_byte_to_utf16_offsets_empty() {
        let result = batch_byte_to_utf16_offsets("hello", [].into_iter());
        assert!(result.is_empty());
    }

    #[test]
    fn test_byte_offset_to_char_offset() {
        assert_eq!(byte_offset_to_char_offset("hello", 0), 0);
        assert_eq!(byte_offset_to_char_offset("hello", 2), 2);
        assert_eq!(byte_offset_to_char_offset("你好", 3), 1);
        assert_eq!(byte_offset_to_char_offset("你好", 6), 2);
    }

    #[test]
    fn test_utf16_offset_to_char_offset() {
        let text = "你好hello";
        assert_eq!(utf16_offset_to_char_offset(text, 0), 0);
        assert_eq!(utf16_offset_to_char_offset(text, 2), 2);
        assert_eq!(utf16_offset_to_char_offset(text, 7), 7);
    }

    #[test]
    fn test_char_offset_to_line_col() {
        let text = "Hello\nWorld\nTest";
        assert_eq!(char_offset_to_line_col(text, 0), (1, 1));
        assert_eq!(char_offset_to_line_col(text, 6), (2, 1));
        assert_eq!(char_offset_to_line_col(text, 12), (3, 1));
    }

    #[test]
    fn test_char_index_to_byte_offset() {
        assert_eq!(char_index_to_byte_offset("hello", 0), 0);
        assert_eq!(char_index_to_byte_offset("hello", 2), 2);
        assert_eq!(char_index_to_byte_offset("你好", 1), 3);
        assert_eq!(char_index_to_byte_offset("你好", 2), 6);
    }

    #[test]
    fn test_visual_units() {
        assert_eq!(visual_units("i"), 0.55);
        assert_eq!(visual_units("m"), 1.3);
        assert_eq!(visual_units("A"), 1.05);
        assert_eq!(visual_units("5"), 0.9);
        assert_eq!(visual_units("."), 0.5);
        assert_eq!(visual_units("x"), 0.92);
    }

    #[test]
    fn test_estimate_word_visual_width() {
        let w = estimate_word_visual_width("hello", 10.0);
        // h=0.92, e=0.92, l=0.55, l=0.55, o=0.92  =>  3.86 units
        let expected = (3.86_f64 * 10.0 * 0.9).max(10.0 * 0.7);
        assert!((w - expected).abs() < 0.001);
    }
}
