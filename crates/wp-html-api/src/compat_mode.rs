#[derive(Debug, PartialEq, Default)]
pub enum CompatMode {
    /// No-quirks mode document compatability mode.
    ///
    /// > In no-quirks mode, the behavior is (hopefully) the desired behavior
    /// > described by the modern HTML and CSS specifications.
    ///
    /// @see https://developer.mozilla.org/en-US/docs/Web/HTML/Quirks_Mode_and_Standards_Mode
    #[default]
    NoQuirks,

    /// Quirks mode document compatability mode.
    ///
    /// > In quirks mode, layout emulates behavior in Navigator 4 and Internet
    /// > Explorer 5. This is essential in order to support websites that were
    /// > built before the widespread adoption of web standards.
    ///
    /// @see https://developer.mozilla.org/en-US/docs/Web/HTML/Quirks_Mode_and_Standards_Mode
    Quirks,

    LimitedQuirks,
}

impl Into<String> for &CompatMode {
    fn into(self) -> String {
        let s: &str = self.into();
        s.to_string()
    }
}
impl Into<&str> for &CompatMode {
    fn into(self) -> &'static str {
        match self {
            CompatMode::NoQuirks => "no-quirks-mode",
            CompatMode::Quirks => "quirks-mode",
            CompatMode::LimitedQuirks => "limited-quirks",
        }
    }
}
