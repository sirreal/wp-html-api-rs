use super::html_token::HTMLToken;
use std::rc::Rc;

/// Core class used by the HTML processor during HTML parsing
/// for managing the stack of active formatting elements.
///
/// This class is designed for internal use by the HTML processor.
///
/// > Initially, the list of active formatting elements is empty.
/// > It is used to handle mis-nested formatting element tags.
/// >
/// > The list contains elements in the formatting category, and markers.
/// > The markers are inserted when entering applet, object, marquee,
/// > template, td, th, and caption elements, and are used to prevent
/// > formatting from "leaking" into applet, object, marquee, template,
/// > td, th, and caption elements.
/// >
/// > In addition, each element in the list of active formatting elements
/// > is associated with the token for which it was created, so that
/// > further elements can be created for that token if necessary.
///
/// @see https://html.spec.whatwg.org/#list-of-active-formatting-elements
/// @see WP_HTML_Processor
pub(super) struct ActiveFormattingElements {
    /// Holds the stack of active formatting element references.
    stack: Vec<ActiveFormattingElement>,
}
impl ActiveFormattingElements {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Returns how many nodes are currently in the stack of active formatting elements.
    ///
    /// @return int How many node are in the stack of active formatting elements.
    pub fn count(&self) -> usize {
        self.stack.len()
    }

    /// Inserts a "marker" at the end of the list of active formatting elements.
    ///
    /// > The markers are inserted when entering applet, object, marquee,
    /// > template, td, th, and caption elements, and are used to prevent
    /// > formatting from "leaking" into applet, object, marquee, template,
    /// > td, th, and caption elements.
    ///
    /// @see https://html.spec.whatwg.org/#concept-parser-marker
    pub fn insert_marker(&mut self) {
        self.stack.push(ActiveFormattingElement::Marker);
    }

    /// Clears the list of active formatting elements up to the last marker.
    ///
    /// > When the steps below require the UA to clear the list of active formatting elements up to
    /// > the last marker, the UA must perform the following steps:
    /// >
    /// > 1. Let entry be the last (most recently added) entry in the list of active
    /// >    formatting elements.
    /// > 2. Remove entry from the list of active formatting elements.
    /// > 3. If entry was a marker, then stop the algorithm at this point.
    /// >    The list has been cleared up to the last marker.
    /// > 4. Go to step 1.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#clear-the-list-of-active-formatting-elements-up-to-the-last-marker
    pub fn clear_up_to_last_marker(&mut self) {
        while let Some(element) = self.stack.pop() {
            if element == ActiveFormattingElement::Marker {
                break;
            }
        }
    }

    /// Pushes a node onto the stack of active formatting elements.
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#push-onto-the-list-of-active-formatting-elements
    ///
    /// @param WP_HTML_Token $token Push this node onto the stack.
    pub fn push(&mut self, token: Rc<HTMLToken>) {
        self.stack.push(ActiveFormattingElement::Token(token))
    }

    /// Returns the node at the end of the stack of active formatting elements,
    /// if one exists. If the stack is empty, returns null.
    ///
    /// @return WP_HTML_Token|null Last node in the stack of active formatting elements, if one exists, otherwise null.
    pub fn current_node(&self) -> Option<&ActiveFormattingElement> {
        self.stack.last()
    }

    /// Steps through the stack of active formatting elements, starting with the
    /// bottom element (added last) and walking upwards to the one added first.
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
    /// see WP_HTML_Active_Formatting_Elements::walk_down().
    pub fn walk_up(&self) -> impl Iterator<Item = &HTMLToken> {
        self.stack.iter().rev().filter_map(|item| match item {
            ActiveFormattingElement::Token(token) => Some(token.as_ref()),
            _ => None,
        })
    }

    // Internal method that returns the actual ActiveFormattingElement references
    pub(super) fn walk_up_elements(&self) -> impl Iterator<Item = &ActiveFormattingElement> {
        self.stack.iter().rev()
    }

    /// Removes a node from the stack of active formatting elements.
    ///
    /// @param WP_HTML_Token $token Remove this node from the stack, if it's there already.
    /// @return bool Whether the node was found and removed from the stack of active formatting elements.
    pub fn remove_node(&mut self, token: &HTMLToken) -> bool {
        // First find the position without the mutable borrow
        let position = self.stack.iter().rev().position(|item| match item {
            ActiveFormattingElement::Token(item_token) => {
                // Dereference both to compare the actual HTMLToken values
                item_token.as_ref() == token
            }
            _ => false,
        });

        // Then remove the element if found
        if let Some(pos) = position {
            let idx = self.stack.len() - 1 - pos;
            self.stack.remove(idx);
            true
        } else {
            false
        }
    }

    /// Checks if a node exists in the stack of active formatting elements.
    ///
    /// @param WP_HTML_Token $token Check if this node exists in the stack.
    /// @return bool Whether the node exists in the stack of active formatting elements.
    pub fn contains_node(&self, token: &HTMLToken) -> bool {
        self.walk_up_elements().any(|item| match item {
            ActiveFormattingElement::Token(item_token) => {
                // Dereference both to compare the actual HTMLToken values
                item_token.as_ref() == token
            }
            _ => false,
        })
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum ActiveFormattingElement {
    Token(Rc<HTMLToken>),
    Marker,
}
