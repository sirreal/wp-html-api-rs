use crate::{
    html_processor::HTMLToken,
    tag_name::TagName,
    tag_processor::{NodeName, ParsingNamespace},
};
use std::rc::Rc;

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
    // MathML
    (&TagName::MI, &ParsingNamespace::MathML),
    (&TagName::MO, &ParsingNamespace::MathML),
    (&TagName::MN, &ParsingNamespace::MathML),
    (&TagName::MS, &ParsingNamespace::MathML),
    (&TagName::MTEXT, &ParsingNamespace::MathML),
    (&TagName::ANNOTATION_XML, &ParsingNamespace::MathML),
    // SVG
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
    pub stack: Vec<Rc<HTMLToken>>,
}
impl StackOfOpenElements {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn _push(&mut self, element: Rc<HTMLToken>) {
        self.stack.push(element);
    }

    pub(super) fn _pop(&mut self) -> Option<Rc<HTMLToken>> {
        self.stack.pop()
    }

    pub fn current_node(&self) -> Option<&HTMLToken> {
        self.stack.last().map(|rc| rc.as_ref())
    }

    pub fn count(&self) -> usize {
        self.stack.len()
    }

    pub fn contains(&self, tag_name: &TagName) -> bool {
        self.stack.iter().any(|t| {
            if let HTMLToken {
                node_name: NodeName::Tag(tag_on_stack),
                ..
            } = t.as_ref()
            {
                tag_on_stack == tag_name
            } else {
                false
            }
        })
    }

    /// Returns the name of the node at the nth position on the stack
    /// of open elements, or `null` if no such position exists.
    ///
    /// Note that this uses a 1-based index, which represents the
    /// "nth item" on the stack, counting from the top, where the
    /// top-most element is the 1st, the second is the 2nd, etc...
    ///
    /// @param int $nth Retrieve the nth item on the stack, with 1 being
    ///                 the top element, 2 being the second, etc...
    /// @return WP_HTML_Token|null Name of the node on the stack at the given location,
    ///                            or `null` if the location isn't on the stack.
    pub fn at(&self, nth: usize) -> Option<&HTMLToken> {
        self.stack.get(nth - 1).map(|rc| rc.as_ref())
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
        if let Some(token_rc) = self.stack.last() {
            token_rc.as_ref().node_name == *identity
        } else {
            false
        }
    }

    pub fn has_any_h1_to_h6_element_in_scope(&self) -> bool {
        for node in self.walk_up() {
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
        self.stack.iter().map(|rc| rc.as_ref())
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
    pub fn walk_up(&self) -> impl Iterator<Item = &HTMLToken> {
        self.stack.iter().rev().map(|rc| rc.as_ref())
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
                // MathML
                (&TagName::MI, &ParsingNamespace::MathML),
                (&TagName::MO, &ParsingNamespace::MathML),
                (&TagName::MN, &ParsingNamespace::MathML),
                (&TagName::MS, &ParsingNamespace::MathML),
                (&TagName::MTEXT, &ParsingNamespace::MathML),
                (&TagName::ANNOTATION_XML, &ParsingNamespace::MathML),
                // SVG
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
        for node in self.walk_up() {
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
        // Compare each element's contents to the provided token
        self.walk_up().any(|node| *node == *last_entry)
    }

    /// Returns whether a particular element is in select scope.
    ///
    /// This test differs from the others like it, in that its rules are inverted.
    /// Instead of arriving at a match when one of any tag in a termination group
    /// is reached, this one terminates if any other tag is reached.
    ///
    /// > The stack of open elements is said to have a particular element in select scope when it has
    /// > that element in the specific scope consisting of all element types except the following:
    /// >   - optgroup in the HTML namespace
    /// >   - option in the HTML namespace
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-select-scope
    ///
    /// @param string $tag_name Name of tag to check.
    /// @return bool Whether the given element is in SELECT scope.
    pub fn has_element_in_select_scope(&self, tag_name: &TagName) -> bool {
        for node in self.walk_up() {
            if let NodeName::Tag(node_tag_name) = &node.node_name {
                if node_tag_name == tag_name {
                    return true;
                }

                if !matches!(node_tag_name, TagName::OPTION | TagName::OPTGROUP) {
                    return false;
                }
            }
        }

        false
    }

    /// Returns whether a particular element is in list item scope.
    ///
    /// > The stack of open elements is said to have a particular element
    /// > in list item scope when it has that element in the specific scope
    /// > consisting of the following element types:
    /// >
    /// >   - All the element types listed above for the has an element in scope algorithm.
    /// >   - ol in the HTML namespace
    /// >   - ul in the HTML namespace
    ///
    /// @see https://html.spec.whatwg.org/#has-an-element-in-list-item-scope
    ///
    /// @param string $tag_name Name of tag to check.
    /// @return bool Whether given element is in scope.
    pub fn has_element_in_list_item_scope(&self, tag_name: &TagName) -> bool {
        self.has_element_in_specific_scope(
            tag_name,
            &[
                // HTML
                (&TagName::APPLET, &ParsingNamespace::Html),
                (&TagName::BUTTON, &ParsingNamespace::Html),
                (&TagName::CAPTION, &ParsingNamespace::Html),
                (&TagName::HTML, &ParsingNamespace::Html),
                (&TagName::TABLE, &ParsingNamespace::Html),
                (&TagName::TD, &ParsingNamespace::Html),
                (&TagName::TH, &ParsingNamespace::Html),
                (&TagName::MARQUEE, &ParsingNamespace::Html),
                (&TagName::OBJECT, &ParsingNamespace::Html),
                (&TagName::OL, &ParsingNamespace::Html),
                (&TagName::TEMPLATE, &ParsingNamespace::Html),
                (&TagName::UL, &ParsingNamespace::Html),
                // MathML
                (&TagName::MI, &ParsingNamespace::MathML),
                (&TagName::MO, &ParsingNamespace::MathML),
                (&TagName::MN, &ParsingNamespace::MathML),
                (&TagName::MS, &ParsingNamespace::MathML),
                (&TagName::MTEXT, &ParsingNamespace::MathML),
                (&TagName::ANNOTATION_XML, &ParsingNamespace::MathML),
                // SVG
                (&TagName::FOREIGNOBJECT, &ParsingNamespace::Svg),
                (&TagName::DESC, &ParsingNamespace::Svg),
                (&TagName::TITLE, &ParsingNamespace::Svg),
            ],
        )
    }
}
