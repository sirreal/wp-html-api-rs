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
    fn strspn() {
        assert_eq!(strspn!(b"abc", b'a' | b'b'), 2);
        assert_eq!(strspn!(b"abc", b'a' | b'b', 1), 1);
        assert_eq!(strspn!(b"abc", b'a' | b'b', 2), 0);
        assert_eq!(strspn!(b"abc", b'a' | b'b', 0, 3), 2);
        assert_eq!(strspn!(b"bc", b'a'), 0);
        assert_eq!(strspn!(b"baaaa", b'a'), 0);
        assert_eq!(strspn!(b"aaaab", b'a'), 4);
        assert_eq!(strspn!(b"aaaab", b'a', 2), 2);
        assert_eq!(strspn!(b"aaaab", b'a', 2, 1), 1);
    }

    #[test]
    fn strcspn() {
        assert_eq!(strcspn!(b"abc", b'a' | b'b'), 0);
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 1), 0);
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 2), 1);
        assert_eq!(strcspn!(b"abc", b'a' | b'b', 0, 3), 0);
        assert_eq!(strcspn!(b"bc", b'a'), 2);
        assert_eq!(strcspn!(b"baaaa", b'a'), 1);
        assert_eq!(strcspn!(b"1234567890", b'0'), 9);
        assert_eq!(strcspn!(b"1234567890", b'0', 1), 8);
        assert_eq!(strcspn!(b"1234567890", b'0', 5), 4);
        assert_eq!(strcspn!(b"1234567890", b'0', 5, 2), 2);
        assert_eq!(strcspn!(b"1234567890", b'0', 8, 2), 1);
    }
}
