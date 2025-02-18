macro_rules! strspn {
    ($expression:expr, $pattern:pat $(if $guard:expr)?) => {{
        $expression
            .iter()
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or(0)
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr) => {{
        $expression[$offset..]
            .iter()
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or(0)
    }};

    ($expression:expr, $pattern:pat $(if $guard:expr)?, $offset:expr, $length:expr) => {{
        $expression[$offset..$offset + $length]
            .iter()
            .position(|&b| !matches!(b, $pattern $(if $guard)?))
            .unwrap_or(0)
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
