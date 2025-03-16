macro_rules! strspn {
    ($haystack:expr, $needle:literal) => {{
        $haystack.iter()
            .position(|&b| b != $needle)
            .unwrap_or($haystack.len())
    }};

    ($haystack:expr, $needle:literal, $offset:expr) => {
        $haystack.iter()
            .skip($offset)
            .position(|&b| b != $needle)
            .unwrap_or($haystack.len() - $offset)
    };

    ($haystack:expr, $needle:literal, $offset:expr, $length:expr) => {
        $haystack.iter()
            .skip($offset)
            .take($length)
            .position(|&b| b != $needle)
            .unwrap_or($length)
    };


    ($haystack:expr, $pattern:pat $(if $guard:expr)?) => {
        $haystack.iter()
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($haystack.len())
    };

    ($haystack:expr, $pattern:pat $(if $guard:expr)?, $offset:expr) => {
        $haystack.iter()
            .skip($offset)
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($haystack.len() - $offset)
    };

    ($haystack:expr, $pattern:pat $(if $guard:expr)?, $offset:expr, $length:expr) => {
        $haystack.iter()
            .skip($offset)
            .take($length)
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or($length)
    };
}

macro_rules! strcspn {
    ($haystack:expr, $needle:literal) => {
        $haystack.iter()
            .position(|&b| b == $needle)
            .unwrap_or($haystack.len())
    };

    ($haystack:expr, $needle:literal, $offset:expr) => {
        $haystack.iter()
            .skip($offset)
            .position(|&b| b == $needle)
            .unwrap_or($haystack.len() - $offset)
    };

    ($haystack:expr, $needle:literal, $offset:expr, $length:expr) => {
        $haystack.iter()
            .skip($offset)
            .take($length)
            .position(|&b| b == $needle)
            .unwrap_or($length)
    };


    ($haystack:expr, $pattern:pat $(if $guard:expr)?) => {
        $haystack.iter()
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($haystack.len())
    };

    ($haystack:expr, $pattern:pat $(if $guard:expr)?, $offset:expr) => {
        $haystack.iter()
            .skip($offset)
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($haystack.len() - $offset)
    };

    ($haystack:expr, $pattern:pat $(if $guard:expr)?, $offset:expr, $length:expr) => {
        $haystack.iter()
            .skip($offset)
            .take($length)
            .position(|&b| matches!(b, $pattern $(if $guard)?))
            .unwrap_or($length)
    };
}

#[cfg(test)]
mod test {
    mod strspn {
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
                strspn!(b"123456", b @ b'0'..=b'9' if (b'1'..=b'5').contains(&b)),
                5
            );
            assert_eq!(
                strspn!(b"123a56", b @ b'0'..=b'9' if (b'1'..=b'5').contains(&b)),
                3
            );
        }

        #[test]
        fn strspn_edge_cases() {
            assert_eq!(strspn!(b"", b'a'), 0);
            assert_eq!(strspn!(b"aaa", b'a', 3), 0);
            assert_eq!(strspn!(b"aaa", b'a', 0, 0), 0);
        }

        #[quickcheck]
        fn strspn_absurd_is_0(s: Vec<u8>) -> bool {
            strspn!(&s, _any if false ) == 0
        }

        #[quickcheck]
        fn strspn_obvious_is_strlen(s: Vec<u8>) -> bool {
            strspn!(&s, _any) == s.len()
        }
    }

    mod strcspn {

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

        #[quickcheck]
        fn strcspn_absurd_is_strlen(s: Vec<u8>) -> bool {
            strcspn!(&s, _any if false) == s.len()
        }

        #[quickcheck]
        fn strcspn_obvious_is_0(s: Vec<u8>) -> bool {
            strcspn!(&s, _any) == 0
        }
    }
}
