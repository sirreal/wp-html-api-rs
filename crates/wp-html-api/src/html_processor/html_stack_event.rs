use crate::html_processor::HTMLToken;

pub(super) struct HTMLStackEvent {
    pub operation: StackOperation,
    pub token: HTMLToken,
    pub provenance: StackProvenance,
}

#[derive(PartialEq)]
pub(super) enum StackOperation {
    Push,
    Pop,
}
#[derive(PartialEq)]
pub(super) enum StackProvenance {
    Real,
    Virtual,
}
