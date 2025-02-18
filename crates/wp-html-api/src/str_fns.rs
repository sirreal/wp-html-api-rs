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

    let p_end = pattern[p_len - 1];

    for at in offset..(s.len() - p_len) {
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

pub fn stripos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let p_len = pattern.len();

    if p_len == 0 {
        return Some(offset);
    }

    if (offset + p_len) > s.len() {
        return None;
    }

    let p_end = pattern[p_len - 1];

    for at in offset..(s.len() - p_len) {
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
