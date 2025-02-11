use super::html_token::HTMLToken;

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

    pub fn push(&self, unwrap: HTMLToken) {
        todo!()
    }

    /// Returns the node at the end of the stack of active formatting elements,
    /// if one exists. If the stack is empty, returns null.
    ///
    /// @return WP_HTML_Token|null Last node in the stack of active formatting elements, if one exists, otherwise null.
    pub fn current_node(&self) -> Option<&ActiveFormattingElement> {
        self.stack.last()
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum ActiveFormattingElement {
    Token(HTMLToken),
    Marker,
}
