use crate::tag_processor::{NodeName, ParsingNamespace};
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub struct HTMLToken {
    pub(crate) is_root_node: bool,
    pub(crate) is_context_node: bool,

    ///
    /// Name of bookmark corresponding to source of token in input HTML string.
    ///
    /// Having a bookmark name does not imply that the token still exists. It
    /// may be that the source token and underlying bookmark was wiped out by
    /// some modification to the source HTML.
    ///
    /// Using Rc<u32> ensures the bookmark is only released when all references to it are gone.
    ///
    /// @since 6.4.0
    ///
    /// @var string
    ///
    pub bookmark_name: Option<Rc<u32>>,

    /**
     * Name of node; lowercase names such as "marker" are not HTML elements.
     *
     * For HTML elements/tags this value should come from WP_HTML_Processor::get_tag().
     *
     * @since 6.4.0
     *
     * @see WP_HTML_Processor::get_tag()
     *
     * @var string
     */
    pub node_name: NodeName,

    /**
     * Whether node contains the self-closing flag.
     *
     * A node may have a self-closing flag when it shouldn't. This value
     * only reports if the flag is present in the original HTML.
     *
     * @since 6.4.0
     *
     * @see https://html.spec.whatwg.org/#self-closing-flag
     *
     * @var bool
     */
    pub has_self_closing_flag: bool,

    /**
     * Indicates if the element is an HTML element or if it's inside foreign content.
     *
     * @since 6.7.0
     *
     * @var string 'html', 'svg', or 'math'.
     */
    pub namespace: ParsingNamespace,

    /**
     * Indicates which kind of integration point the element is, if any.
     *
     * @since 6.7.0
     *
     * @var string|null 'math', 'html', or null if not an integration point.
     */
    pub integration_node_type: Option<IntegrationNodeType>,
}
impl HTMLToken {
    ///
    /// Constructor - creates a reference to a token in some external HTML string.
    ///
    /// @since 6.4.0
    ///
    /// @param string|null   $bookmark_name         Name of bookmark corresponding to location in HTML where token is found,
    ///                                             or `null` for markers and nodes without a bookmark.
    /// @param string        $node_name             Name of node token represents; if uppercase, an HTML element; if lowercase, a special value like "marker".
    /// @param bool          $has_self_closing_flag Whether the source token contains the self-closing flag, regardless of whether it's valid.
    ///
    pub fn new(
        bookmark_name: Option<Rc<u32>>,
        node_name: NodeName,
        has_self_closing_flag: bool,
    ) -> Self {
        Self {
            bookmark_name,
            node_name,
            has_self_closing_flag,
            ..Default::default()
        }
    }
}

impl Default for HTMLToken {
    fn default() -> Self {
        Self {
            is_root_node: false,
            is_context_node: false,
            bookmark_name: None,
            namespace: Default::default(),
            integration_node_type: None,
            node_name: NodeName::Token(crate::tag_processor::TokenType::Text),
            has_self_closing_flag: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum IntegrationNodeType {
    HTML,
    MathML,
}
