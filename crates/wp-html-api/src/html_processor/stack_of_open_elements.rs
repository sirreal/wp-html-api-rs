use crate::{
    html_processor::HTMLToken,
    tag_processor::{NodeName, TagName},
};

/// Core class used by the HTML processor during HTML parsing
/// for managing the stack of open elements.
///
/// This class is designed for internal use by the HTML processor.
///
/// > Initially, the stack of open elements is empty. The stack grows
/// > downwards; the topmost node on the stack is the first one added
/// > to the stack, and the bottommost node of the stack is the most
/// > recently added node in the stack (notwithstanding when the stack
/// > is manipulated in a random access fashion as part of the handling
/// > for misnested tags).
///
/// @see https://html.spec.whatwg.org/#stack-of-open-elements
/// @see WP_HTML_Processor
pub(super) struct StackOfOpenElements {
    /// Holds the stack of open element references.
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

    /// Returns whether a P is in BUTTON scope.
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-button-scope
    ///
    /// @return bool Whether a P is in BUTTON scope.
    pub(crate) fn has_p_in_button_scope(&self) -> bool {
        self.has_element_in_button_scope(&TagName::P)
    }

    pub(crate) fn current_node_is(&self, tag_name: &TagName) -> bool {
        todo!()
    }

    pub(crate) fn has_any_h1_to_h6_element_in_scope(&self) -> bool {
        todo!()
    }

    pub(crate) fn pop_until_any_h1_to_h6(&self) {
        todo!()
    }
}
