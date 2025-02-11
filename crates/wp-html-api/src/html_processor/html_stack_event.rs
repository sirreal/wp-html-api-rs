use crate::html_processor::HTMLToken;

#[derive(Debug)]
pub(super) struct HTMLStackEvent {
    pub operation: StackOperation,
    pub token: HTMLToken,
    pub provenance: StackProvenance,
}

#[derive(Debug, PartialEq)]
pub(super) enum StackOperation {
    Push,
    Pop,
}
#[derive(Debug, PartialEq)]
pub(super) enum StackProvenance {
    Real,
    Virtual,
}
