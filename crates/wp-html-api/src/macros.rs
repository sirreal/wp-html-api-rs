macro_rules! strspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| !matches!(b, $pattern))
            .unwrap_or(0)
    }};
}

macro_rules! strcspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| matches!(b, $pattern))
            .unwrap_or($expression.len() - $offset)
    }};
}
