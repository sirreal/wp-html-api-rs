#[macro_export]
macro_rules! strspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| !matches!(b, $pattern))
            .unwrap_or(0)
    }};
}

#[macro_export]
macro_rules! strcspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| matches!(b, $pattern))
            .unwrap_or($expression.len() - $offset)
    }};
}

pub fn substr(s: &[u8], offset: usize, length: usize) -> &[u8] {
    &s[offset..offset + length]
}

pub fn strpos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let p_len = pattern.len();

    if p_len == 0 {
        return Some(offset);
    }

    if (offset + p_len) > s.len() {
        return None;
    }

    let p_end = pattern.get(p_len - 1).unwrap();

    for at in offset..s.len() {
        let c = s.get(at + p_len - 1).unwrap();

        if c != p_end {
            continue;
        }

        if &s[at..(at + p_len)] == pattern {
            return Some(at);
        }
    }

    None
}

pub fn stripos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let p_len = pattern.len();

    if p_len == 0 {
        return Some(offset);
    }

    if (offset + p_len) > s.len() {
        return None;
    }

    let p_end = pattern.get(p_len - 1).unwrap();

    for at in offset..s.len() {
        let c = s.get(at + p_len - 1).unwrap();

        if !p_end.eq_ignore_ascii_case(&c) {
            continue;
        }

        if pattern.eq_ignore_ascii_case(&s[at..(at + p_len)]) {
            return Some(at);
        }
    }

    None
}
