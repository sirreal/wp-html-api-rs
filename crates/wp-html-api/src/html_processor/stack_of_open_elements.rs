use crate::{
    html_processor::HTMLToken,
    tag_name::TagName,
    tag_processor::{NodeName, ParsingNamespace},
};

const ELEMENT_IN_SCOPE_TERMINATION_LIST: [(&TagName, &ParsingNamespace); 18] = [
    (&TagName::APPLET, &ParsingNamespace::Html),
    (&TagName::CAPTION, &ParsingNamespace::Html),
    (&TagName::HTML, &ParsingNamespace::Html),
    (&TagName::TABLE, &ParsingNamespace::Html),
    (&TagName::TD, &ParsingNamespace::Html),
    (&TagName::TH, &ParsingNamespace::Html),
    (&TagName::MARQUEE, &ParsingNamespace::Html),
    (&TagName::OBJECT, &ParsingNamespace::Html),
    (&TagName::TEMPLATE, &ParsingNamespace::Html),
    (&TagName::MI, &ParsingNamespace::MathML),
    (&TagName::MO, &ParsingNamespace::MathML),
    (&TagName::MN, &ParsingNamespace::MathML),
    (&TagName::MS, &ParsingNamespace::MathML),
    (&TagName::MTEXT, &ParsingNamespace::MathML),
    (&TagName::ANNOTATION_XML, &ParsingNamespace::MathML),
    (&TagName::FOREIGNOBJECT, &ParsingNamespace::Svg),
    (&TagName::DESC, &ParsingNamespace::Svg),
    (&TagName::TITLE, &ParsingNamespace::Svg),
];

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

    pub fn _push(&mut self, element: HTMLToken) {
        self.stack.push(element.clone());
    }

    pub fn _pop(&mut self) -> Option<HTMLToken> {
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

    pub fn at(&self, nth: usize) -> Option<HTMLToken> {
        todo!()
    }

    /// Returns whether a particular element is in table scope.
    ///
    /// > The stack of open elements is said to have a particular element
    /// > in table scope when it has that element in the specific scope
    /// > consisting of the following element types:
    /// >
    /// >   - html in the HTML namespace
    /// >   - table in the HTML namespace
    /// >   - template in the HTML namespace
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-table-scope
    pub fn has_element_in_table_scope(&self, tag_name: &TagName) -> bool {
        self.has_element_in_specific_scope(
            tag_name,
            &[
                (&TagName::HTML, &ParsingNamespace::Html),
                (&TagName::TABLE, &ParsingNamespace::Html),
                (&TagName::TEMPLATE, &ParsingNamespace::Html),
            ],
        )
    }

    /// Returns whether a particular element is in scope.
    ///
    /// > The stack of open elements is said to have a particular element in
    /// > scope when it has that element in the specific scope consisting of
    /// > the following element types:
    /// >
    /// >   - applet
    /// >   - caption
    /// >   - html
    /// >   - table
    /// >   - td
    /// >   - th
    /// >   - marquee
    /// >   - object
    /// >   - template
    /// >   - MathML mi
    /// >   - MathML mo
    /// >   - MathML mn
    /// >   - MathML ms
    /// >   - MathML mtext
    /// >   - MathML annotation-xml
    /// >   - SVG foreignObject
    /// >   - SVG desc
    /// >   - SVG title
    pub fn has_element_in_scope(&self, tag_name: &TagName) -> bool {
        self.has_element_in_specific_scope(tag_name, &ELEMENT_IN_SCOPE_TERMINATION_LIST)
    }

    /// Returns whether a P is in BUTTON scope.
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-button-scope
    ///
    /// @return bool Whether a P is in BUTTON scope.
    pub fn has_p_in_button_scope(&self) -> bool {
        self.has_element_in_button_scope(&TagName::P)
    }

    /// Indicates if the current node is of a given type or name.
    ///
    /// It's possible to pass either a node type or a node name to this function.
    /// In the case there is no current element it will always return `false`.
    ///
    /// Example:
    ///
    ///     // Is the current node a text node?
    ///     $stack->current_node_is( '#text' );
    ///
    ///     // Is the current node a DIV element?
    ///     $stack->current_node_is( 'DIV' );
    ///
    ///     // Is the current node any element/tag?
    ///     $stack->current_node_is( '#tag' );
    ///
    /// @see WP_HTML_Tag_Processor::get_token_type
    /// @see WP_HTML_Tag_Processor::get_token_name
    ///
    /// @since 6.7.0
    ///
    /// @access private
    ///
    /// @param string $identity Check if the current node has this name or type (depending on what is provided).
    /// @return bool Whether there is a current element that matches the given identity, whether a token name or type.
    pub fn current_node_is(&self, identity: &NodeName) -> bool {
        if let Some(HTMLToken { node_name, .. }) = self.stack.last() {
            node_name == identity
        } else {
            false
        }
    }

    pub(super) fn has_any_h1_to_h6_element_in_scope(&self) -> bool {
        for node in self.walk_up(None) {
            if let HTMLToken {
                node_name: NodeName::Tag(node_tag),
                namespace,
                ..
            } = node
            {
                if matches!(
                    node_tag,
                    TagName::H1
                        | TagName::H2
                        | TagName::H3
                        | TagName::H4
                        | TagName::H5
                        | TagName::H6
                ) {
                    return true;
                }

                if ELEMENT_IN_SCOPE_TERMINATION_LIST.contains(&(node_tag, namespace)) {
                    return false;
                }
            }
        }
        // If we've walked through the entire stack without finding the tag, it's not in scope
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
                (&TagName::APPLET, &ParsingNamespace::Html),
                (&TagName::BUTTON, &ParsingNamespace::Html),
                (&TagName::CAPTION, &ParsingNamespace::Html),
                (&TagName::HTML, &ParsingNamespace::Html),
                (&TagName::TABLE, &ParsingNamespace::Html),
                (&TagName::TD, &ParsingNamespace::Html),
                (&TagName::TH, &ParsingNamespace::Html),
                (&TagName::MARQUEE, &ParsingNamespace::Html),
                (&TagName::OBJECT, &ParsingNamespace::Html),
                (&TagName::TEMPLATE, &ParsingNamespace::Html),
                (&TagName::MI, &ParsingNamespace::MathML),
                (&TagName::MO, &ParsingNamespace::MathML),
                (&TagName::MN, &ParsingNamespace::MathML),
                (&TagName::MS, &ParsingNamespace::MathML),
                (&TagName::MTEXT, &ParsingNamespace::MathML),
                (&TagName::ANNOTATION_XML, &ParsingNamespace::MathML),
                (&TagName::FOREIGNOBJECT, &ParsingNamespace::Svg),
                (&TagName::DESC, &ParsingNamespace::Svg),
                (&TagName::TITLE, &ParsingNamespace::Svg),
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
        termination_list: &[(&TagName, &ParsingNamespace)],
    ) -> bool {
        for node in self.walk_up(None) {
            if let HTMLToken {
                node_name: NodeName::Tag(node_tag),
                namespace,
                ..
            } = node
            {
                if node_tag == tag_name {
                    return true;
                }
                if termination_list.contains(&(node_tag, namespace)) {
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
