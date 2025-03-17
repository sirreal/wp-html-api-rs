use memchr::memchr;

/// substr — Return part of a string
/// See https://www.php.net/manual/en/function.substr.php
pub fn substr(s: &[u8], offset: usize, length: usize) -> &[u8] {
    &s[offset..offset + length]
}

/// strpos — Find the position of the first occurrence of a substring in a string
/// See https://www.php.net/manual/en/function.strpos.php
pub fn strpos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let p_len = pattern.len();

    if p_len == 0 {
        return Some(offset);
    }

    if (offset + p_len) > s.len() {
        return None;
    }

    let p_end = pattern[p_len - 1];

    for at in offset..=(s.len() - p_len) {
        let c = s[at + p_len - 1];

        if c != p_end {
            continue;
        }

        if &s[at..(at + p_len)] == pattern {
            return Some(at);
        }
    }

    None
}

/// strpos — Find the position of the first occurrence of a substring in a string
/// See https://www.php.net/manual/en/function.strpos.php
pub fn strpos_byte(s: &[u8], pattern: u8, offset: usize) -> Option<usize> {
    if offset > s.len() {
        None
    } else {
        memchr(pattern, &s[offset..]).map(|pos| pos + offset)
    }
}

/// stripos — Find the position of the first occurrence of a case-insensitive substring in a string
/// See https://www.php.net/manual/en/function.stripos.php
pub fn stripos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let p_len = pattern.len();

    if p_len == 0 {
        return Some(offset);
    }

    if (offset + p_len) > s.len() {
        return None;
    }

    let p_end = pattern[p_len - 1];

    for at in offset..=(s.len() - p_len) {
        let c = s[at + p_len - 1];

        if !p_end.eq_ignore_ascii_case(&c) {
            continue;
        }

        if pattern.eq_ignore_ascii_case(&s[at..(at + p_len)]) {
            return Some(at);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_substr() {
        let s = b"Hello, World!";
        assert_eq!(substr(s, 0, 5), b"Hello");
        assert_eq!(substr(s, 7, 5), b"World");
        assert_eq!(substr(s, 0, s.len()), s);
        assert_eq!(substr(s, 5, 2), b", ");
    }

    #[test]
    #[should_panic]
    fn test_substr_out_of_bounds() {
        let s = b"Hello";
        substr(s, 0, 6); // Should panic - length too long
    }

    #[test]
    fn test_strpos() {
        let s = b"Hello, World!";
        assert_eq!(strpos(s, b"Hello", 0), Some(0));
        assert_eq!(strpos(s, b"World", 0), Some(7));
        assert_eq!(strpos(s, b"!", 0), Some(12));
        assert_eq!(strpos(s, b"o", 0), Some(4));
        assert_eq!(strpos(s, b"o", 5), Some(8));
        assert_eq!(strpos(s, b"xyz", 0), None);
        assert_eq!(strpos(s, b"", 0), Some(0));
        assert_eq!(strpos(s, b"Hello", 1), None);
        assert_eq!(strpos(b"", b"", 0), Some(0));
        assert_eq!(strpos(b"", b"x", 0), None);
    }

    #[test]
    fn test_stripos() {
        let s = b"Hello, World!";
        assert_eq!(stripos(s, b"HELLO", 0), Some(0));
        assert_eq!(stripos(s, b"world", 0), Some(7));
        assert_eq!(stripos(s, b"World", 0), Some(7));
        assert_eq!(stripos(s, b"O", 0), Some(4));
        assert_eq!(stripos(s, b"o", 5), Some(8));
        assert_eq!(stripos(s, b"XYZ", 0), None);
        assert_eq!(stripos(s, b"", 0), Some(0));
        assert_eq!(stripos(s, b"HELLO", 1), None);
        assert_eq!(stripos(b"", b"", 0), Some(0));
        assert_eq!(stripos(b"", b"x", 0), None);

        // Mixed case tests
        let mixed = b"aBcDeFgHiJkL";
        assert_eq!(stripos(mixed, b"DEFG", 0), Some(3));
        assert_eq!(stripos(mixed, b"defg", 0), Some(3));
        assert_eq!(stripos(mixed, b"DeFg", 0), Some(3));
    }

    #[test]
    fn test_boundary_conditions() {
        let s = b"test";
        // Offset at end of string
        assert_eq!(strpos(s, b"t", 4), None);
        assert_eq!(stripos(s, b"t", 4), None);

        // Pattern longer than remaining string
        assert_eq!(strpos(s, b"test!", 0), None);
        assert_eq!(stripos(s, b"TEST!", 0), None);

        // Offset beyond string length
        assert_eq!(strpos(s, b"t", 5), None);
        assert_eq!(stripos(s, b"t", 5), None);
    }
}
