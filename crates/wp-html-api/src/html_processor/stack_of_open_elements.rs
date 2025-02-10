use crate::{
    html_processor::HTMLToken,
    tag_processor::{NodeName, TagName},
};

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

    pub(crate) fn current_node(&self) -> Option<&HTMLToken> {
        self.stack.last()
    }

    pub(crate) fn count(&self) -> usize {
        self.stack.len()
    }

    pub(crate) fn contains(&self, tag_name: &TagName) -> bool {
        self.stack.iter().any(|t| {
            if let HTMLToken {
                node_name: NodeName::Tag(tag_on_stack),
                ..
            } = t
            {
                tag_on_stack == tag_name
            } else {
                false
            }
        })
    }

    pub(crate) fn pop_until(&self, tag_name: &TagName) -> bool {
        todo!()
    }

    pub(crate) fn at(&self, nth: usize) -> Option<HTMLToken> {
        todo!()
    }

    pub(crate) fn has_element_in_scope(&self, body: &TagName) -> bool {
        todo!()
    }

    pub(crate) fn has_p_in_button_scope(&self) -> bool {
        todo!()
    }

    pub(crate) fn current_node_is(&self, tag_name: &TagName) -> bool {
        todo!()
    }
}
