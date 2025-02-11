use crate::{
    html_processor::HTMLToken,
    tag_processor::{NodeName, ParsingNamespace, TagName},
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

    push_handler: Option<Box<dyn FnMut(HTMLToken) -> ()>>,
    pop_handler: Option<Box<dyn FnMut(HTMLToken) -> ()>>,
}
impl StackOfOpenElements {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            push_handler: None,
            pop_handler: None,
        }
    }

    pub fn set_push_handler(&mut self, handler: Box<dyn FnMut(HTMLToken) -> ()>) {
        self.push_handler = Some(handler);
    }

    pub fn set_pop_handler(&mut self, handler: Box<dyn FnMut(HTMLToken) -> ()>) {
        self.pop_handler = Some(handler);
    }

    pub fn push(&mut self, element: HTMLToken) {
        self.stack.push(element)
    }

    pub fn pop(&mut self) -> Option<HTMLToken> {
        self.stack.pop()
    }

    pub fn current_node(&self) -> Option<&HTMLToken> {
        self.stack.last()
    }

    pub fn count(&self) -> usize {
        self.stack.len()
    }

    pub fn contains(&self, tag_name: &TagName) -> bool {
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

    /// Pops nodes off of the stack of open elements until an HTML tag with the given name has been popped.
    ///
    /// @see WP_HTML_Open_Elements::pop
    ///
    /// @param string $html_tag_name Name of tag that needs to be popped off of the stack of open elements.
    /// @return bool Whether a tag of the given name was found and popped off of the stack of open elements.
    pub fn pop_until(&mut self, html_tag_name: &TagName) -> bool {
        while let Some(HTMLToken {
            node_name: token_node_name,
            namespace: token_namespace,
            ..
        }) = self.pop()
        {
            if token_namespace != ParsingNamespace::Html {
                continue;
            }

            match token_node_name {
                NodeName::Tag(tag_name) => {
                    if tag_name == *html_tag_name {
                        return true;
                    }
                }
                NodeName::Token(_) => {}
            }
        }

        false
    }

    pub fn at(&self, nth: usize) -> Option<HTMLToken> {
        todo!()
    }

    pub fn has_element_in_scope(&self, body: &TagName) -> bool {
        todo!()
    }

    /// Returns whether a P is in BUTTON scope.
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-button-scope
    ///
    /// @return bool Whether a P is in BUTTON scope.
    pub fn has_p_in_button_scope(&self) -> bool {
        self.has_element_in_button_scope(&TagName::P)
    }

    pub fn current_node_is(&self, tag_name: &TagName) -> bool {
        todo!()
    }

    pub fn has_any_h1_to_h6_element_in_scope(&self) -> bool {
        todo!()
    }

    /// Pop until any H1-H6 element has been popped off of the stack of open elements.
    ///
    /// !!! This function does not exist in the PHP implementation !!!
    ///
    /// Most pop_until usage is for a single element. The H1-H6 elements are an
    /// exception and this additional method prevents needing to implement checks for multiple
    /// elements.
    ///
    /// The
    pub fn pop_until_any_h1_to_h6(&mut self) -> bool {
        while let Some(HTMLToken {
            node_name: token_node_name,
            namespace: token_namespace,
            ..
        }) = self.pop()
        {
            if token_namespace != ParsingNamespace::Html {
                continue;
            }

            if matches!(
                token_node_name,
                NodeName::Tag(
                    TagName::H1
                        | TagName::H2
                        | TagName::H3
                        | TagName::H4
                        | TagName::H5
                        | TagName::H6
                )
            ) {
                return true;
            }
        }

        false
    }

    /// Steps through the stack of open elements, starting with the top element
    /// (added first) and walking downwards to the one added last.
    ///
    /// This generator function is designed to be used inside a "foreach" loop.
    ///
    /// Example:
    ///
    ///     $html = '<em><strong><a>We are here';
    ///     foreach ( $stack->walk_down() as $node ) {
    ///         echo "{$node->node_name} -> ";
    ///     }
    ///     > EM -> STRONG -> A ->
    ///
    /// To start with the most-recently added element and walk towards the top,
    /// see WP_HTML_Open_Elements::walk_up().
    pub fn walk_down(&self) -> impl Iterator<Item = &HTMLToken> {
        self.stack.iter()
    }

    /// Steps through the stack of open elements, starting with the bottom element
    /// (added last) and walking upwards to the one added first.
    ///
    /// This generator function is designed to be used inside a "foreach" loop.
    ///
    /// Example:
    ///
    ///     $html = '<em><strong><a>We are here';
    ///     foreach ( $stack->walk_up() as $node ) {
    ///         echo "{$node->node_name} -> ";
    ///     }
    ///     > A -> STRONG -> EM ->
    ///
    /// To start with the first added element and walk towards the bottom,
    /// see WP_HTML_Open_Elements::walk_down().
    ///
    /// @param WP_HTML_Token|null $above_this_node Optional. Start traversing above this node,
    ///                                            if provided and if the node exists.
    pub fn walk_up(&self, above_this_node: Option<&HTMLToken>) -> impl Iterator<Item = &HTMLToken> {
        if above_this_node.is_some() {
            todo!("Above this node not implemented");
        }

        self.stack.iter().rev()
    }

    /// Returns whether a particular element is in button scope.
    ///
    /// > The stack of open elements is said to have a particular element
    /// > in button scope when it has that element in the specific scope
    /// > consisting of the following element types:
    /// >
    /// >   - All the element types listed above for the has an element in scope algorithm.
    /// >   - button in the HTML namespace
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-button-scope
    ///
    /// @param string $tag_name Name of tag to check.
    /// @return bool Whether given element is in scope.
    fn has_element_in_button_scope(&self, tag_name: &TagName) -> bool {
        self.has_element_in_specific_scope(
            tag_name,
            &[
                TagName::APPLET,
                TagName::BUTTON,
                TagName::CAPTION,
                TagName::HTML,
                TagName::TABLE,
                TagName::TD,
                TagName::TH,
                TagName::MARQUEE,
                TagName::OBJECT,
                TagName::TEMPLATE,
                TagName::MathML_MI,
                TagName::MathML_MO,
                TagName::MathML_MN,
                TagName::MathML_MS,
                TagName::MathML_MTEXT,
                TagName::MathML_ANNOTATION_XML,
                TagName::SVG_FOREIGNOBJECT,
                TagName::SVG_DESC,
                TagName::SVG_TITLE,
            ],
        )
    }

    /// Returns whether an element is in a specific scope.
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-the-specific-scope
    ///
    /// @param string   $tag_name         Name of tag check.
    /// @param string[] $termination_list List of elements that terminate the search.
    /// @return bool Whether the element was found in a specific scope.
    fn has_element_in_specific_scope(
        &self,
        tag_name: &TagName,
        termination_list: &[TagName],
    ) -> bool {
        for node in self.walk_up(None) {
            if let HTMLToken {
                node_name: NodeName::Tag(node_tag),
                ..
            } = node
            {
                if node_tag == tag_name {
                    return true;
                }
                if termination_list.contains(node_tag) {
                    return false;
                }
            }
        }
        // If we've walked through the entire stack without finding the tag, it's not in scope
        false
    }

    /// Reports if a specific node is in the stack of open elements.
    ///
    /// @param WP_HTML_Token $token Look for this node in the stack.
    /// @return bool Whether the referenced node is in the stack of open elements.
    pub fn contains_node(&self, last_entry: &HTMLToken) -> bool {
        self.walk_up(None).any(|node| node == last_entry)
    }
}
