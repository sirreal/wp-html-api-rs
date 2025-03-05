#[derive(Clone, Copy, Debug)]
pub enum HtmlProcessorError {
    ExceededMaxBookmarks,
    UnsupportedException(UnsupportedException),
}
impl std::error::Error for HtmlProcessorError {
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
}
impl std::fmt::Display for HtmlProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into())
    }
}
impl Into<&str> for HtmlProcessorError {
    fn into(self) -> &'static str {
        match self {
            HtmlProcessorError::ExceededMaxBookmarks => "exceeded-max-bookmarks",
            HtmlProcessorError::UnsupportedException(_) => "unsupported",
        }
    }
}
impl Into<&str> for &HtmlProcessorError {
    fn into(self) -> &'static str {
        Into::<&str>::into(*self)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UnsupportedException {
    MetaTagCharsetDetermineEncoding,
    MetaTagHttpEquivDetermineEncoding,
    AfterHeadElementsReopenHead,
    CannotProcessNonIgnoredFrameset,
    CannotProcessPlaintextElements,
    FosterParenting,
    ContentOutsideOfBody,
    ActiveFormattingElementsWhenAdvancingAndRewindingIsRequired,
    AdoptionAgencyWhenAnyOtherEndTagIsRequired,
    AdoptionAgencyCannotExtractCommonAncestor,
    AdoptionAgencyWhenLoopingRequired,
    ContentOutsideOfHtml,
    NonWhitespaceTextInFrameset,
    NonWhitespaceCharsAfterFrameset,
    NonWhitespaceCharsAfterAfterFrameset,
    CannotCloseFormWithOtherElementsOpen,
}
impl std::fmt::Display for UnsupportedException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into())
    }
}

impl Into<&str> for UnsupportedException {
    fn into(self) -> &'static str {
        use super::UnsupportedException as E;
        match self {
            E::MetaTagCharsetDetermineEncoding => {
                        "Cannot yet process META tags with charset to determine encoding."
                    }
            E::MetaTagHttpEquivDetermineEncoding => {
                        "Cannot yet process META tags with http-equiv Content-Type to determine encoding."
                    }
            E::AfterHeadElementsReopenHead => {
                        "Cannot process elements after HEAD which reopen the HEAD element."
                    }
            E::CannotProcessNonIgnoredFrameset => "Cannot process non-ignored FRAMESET tags.",
            E::CannotProcessPlaintextElements => "Cannot process PLAINTEXT elements.",
            E::FosterParenting => "Foster parenting is not supported.",
            E::ContentOutsideOfBody => "Content outside of BODY is unsupported.",
            E::ActiveFormattingElementsWhenAdvancingAndRewindingIsRequired => "Cannot reconstruct active formatting elements when advancing and rewinding is required." ,
            E::AdoptionAgencyWhenAnyOtherEndTagIsRequired =>                                    "Cannot run adoption agency when \"any other end tag\" is required.",
            E::AdoptionAgencyCannotExtractCommonAncestor => "Cannot extract common ancestor in adoption agency algorithm.",
            E::AdoptionAgencyWhenLoopingRequired => "Cannot run adoption agency when looping required.",
            E::ContentOutsideOfHtml => "Content outside of HTML is unsupported.",
            E::NonWhitespaceTextInFrameset =>"Non-whitespace characters cannot be handled in frameset." ,
            E::NonWhitespaceCharsAfterFrameset => "Non-whitespace characters cannot be handled in after frameset",
            E::NonWhitespaceCharsAfterAfterFrameset => "Non-whitespace characters cannot be handled in after after frameset.",
            E::CannotCloseFormWithOtherElementsOpen => "Cannot close a FORM when other elements remain open as this would throw off the breadcrumbs for the following tokens."
        }
    }
}
impl Into<&str> for &UnsupportedException {
    fn into(self) -> &'static str {
        Into::<&str>::into(*self)
    }
}
