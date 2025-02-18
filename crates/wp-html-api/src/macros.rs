macro_rules! strspn {
    ($expression:expr, $pattern:pat $(if $guard:expr)?) => {{
        $expression
            .iter()
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($expression.len())
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr) => {{
        $expression
            .iter()
            .skip($offset)
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($expression.len() - $offset)
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr, $length:expr) => {{
        $expression
            .iter()
            .skip($offset)
            .take($length)
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($length)
    }};
}

macro_rules! strcspn {
    ($expression:expr, $pattern:pat $(if $guard:expr)?) => {{
        $expression
            .iter()
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($expression.len())
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($expression.len() - $offset)
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr, $length:expr) => {{
        $expression[$offset..$offset + $length]
            .iter()
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($length)
    }};
}

#[cfg(test)]
mod test {
    #[test]
    fn strspn_basic() {
        assert_eq!(strspn!(b"abc", b'a' | b'b'), 2);
        assert_eq!(strspn!(b"bc", b'a'), 0);
        assert_eq!(strspn!(b"baaaa", b'a'), 0);
        assert_eq!(strspn!(b"aaaab", b'a'), 4);
    }

    #[test]
    fn strspn_with_offset() {
        assert_eq!(strspn!(b"abc", b'a' | b'b', 1), 1);
        assert_eq!(strspn!(b"abc", b'a' | b'b', 2), 0);
        assert_eq!(strspn!(b"aaaab", b'a', 2), 2);
    }

    #[test]
    fn strspn_with_length() {
        assert_eq!(strspn!(b"abc", b'a' | b'b', 0, 3), 2);
        assert_eq!(strspn!(b"aaaab", b'a', 2, 1), 1);
        assert_eq!(strspn!(b"aaaab", b'a', 0, 3), 3);
    }

    #[test]
    fn strspn_with_guard() {
        assert_eq!(
            strspn!(b"123456", b @ b'0'..=b'9' if b >= b'1' && b <= b'5'),
            5
        );
        assert_eq!(
            strspn!(b"123a56", b @ b'0'..=b'9' if b >= b'1' && b <= b'5'),
            3
        );
    }

    #[test]
    fn strspn_edge_cases() {
        assert_eq!(strspn!(b"", b'a'), 0);
        assert_eq!(strspn!(b"aaa", b'a', 3), 0);
        assert_eq!(strspn!(b"aaa", b'a', 0, 0), 0);
    }

    #[test]
    fn strcspn_basic() {
        assert_eq!(strcspn!(b"abc", b'a' | b'b'), 0);
        assert_eq!(strcspn!(b"bc", b'a'), 2);
        assert_eq!(strcspn!(b"baaaa", b'a'), 1);
        assert_eq!(strcspn!(b"1234567890", b'0'), 9);
    }

    #[test]
    fn strcspn_with_offset() {
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 1), 0);
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 2), 1);
        assert_eq!(strcspn!(b"1234567890", b'0', 1), 8);
        assert_eq!(strcspn!(b"1234567890", b'0', 5), 4);
    }

    #[test]
    fn strcspn_with_length() {
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 0, 3), 0);
        assert_eq!(strcspn!(b"1234567890", b'0', 5, 2), 2);
        assert_eq!(strcspn!(b"1234567890", b'0', 8, 2), 1);
    }

    #[test]
    fn strcspn_with_guard() {
        assert_eq!(strcspn!(b"123456", b @ b'0'..=b'9' if b >= b'6'), 5);
        assert_eq!(strcspn!(b"12a456", b @ b'0'..=b'9' if b >= b'4'), 3);
    }

    #[test]
    fn strcspn_edge_cases() {
        assert_eq!(strcspn!(b"", b'a'), 0);
        assert_eq!(strcspn!(b"aaa", b'a', 3), 0);
        assert_eq!(strcspn!(b"aaa", b'a', 0, 0), 0);
        assert_eq!(strcspn!(b"abc", b'z'), 3);
    }
}
