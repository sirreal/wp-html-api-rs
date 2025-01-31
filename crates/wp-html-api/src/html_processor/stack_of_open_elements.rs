use crate::html_processor::HTMLToken;

pub(super) struct StackOfOpenElements {
    pub stack: Vec<HTMLToken>,
}
impl StackOfOpenElements {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, element: HTMLToken) {
        self.stack.push(element)
    }

    pub fn pop(&mut self) -> Option<HTMLToken> {
        self.stack.pop()
    }
}
