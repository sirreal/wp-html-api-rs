use crate::html_processor::HTMLToken;

pub(super) struct StackOfOpenElements {}
impl StackOfOpenElements {
    pub fn new() -> Self {
        Self {}
    }

    pub fn push(&mut self, element: HTMLToken) {
        todo!()
    }
}
