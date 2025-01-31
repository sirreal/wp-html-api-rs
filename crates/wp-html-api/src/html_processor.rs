#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use crate::tag_processor::{CommentType, ParsingNamespace, TagName, TagProcessor, TokenType};

enum IntegrationNodeType {
    HTML,
    MathML,
}

struct StackOfOpenElements {}
impl StackOfOpenElements {
    fn new() -> Self {
        Self {}
    }

    fn push(&mut self, element: HTMLToken) {
        todo!()
    }
}
struct ActiveFormattingElements {}
impl ActiveFormattingElements {
    fn new() -> Self {
        Self {}
    }
}
struct HTMLToken {
    ///
    /// Name of bookmark corresponding to source of token in input HTML string.
    ///
    /// Having a bookmark name does not imply that the token still exists. It
    /// may be that the source token and underlying bookmark was wiped out by
    /// some modification to the source HTML.
    ///
    /// @since 6.4.0
    ///
    /// @var string
    ///
    bookmark_name: Option<Box<str>>,

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
    node_name: Box<str>,

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
    has_self_closing_flag: bool,

    /**
     * Indicates if the element is an HTML element or if it's inside foreign content.
     *
     * @since 6.7.0
     *
     * @var string 'html', 'svg', or 'math'.
     */
    namespace: ParsingNamespace,

    /**
     * Indicates which kind of integration point the element is, if any.
     *
     * @since 6.7.0
     *
     * @var string|null 'math', 'html', or null if not an integration point.
     */
    integration_node_type: Option<IntegrationNodeType>,
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
    /// @param callable|null $on_destroy            Optional. Function to call when destroying token, useful for releasing the bookmark.
    ///
    pub fn new(bookmark_name: Option<&str>, node_name: &str, has_self_closing_flag: bool) -> Self {
        Self {
            bookmark_name: bookmark_name.map(|s| s.into()),
            namespace: Default::default(),
            integration_node_type: None,
            node_name: node_name.into(),
            has_self_closing_flag,
        }
    }
}

pub struct ProcessorState {
    active_formatting_elements: ActiveFormattingElements,
    current_token: Option<HTMLToken>,
    encoding: Box<str>,
    encoding_confidence: EncodingConfidence,
    form_element: Option<HTMLToken>,
    frameset_ok: bool,
    head_element: Option<HTMLToken>,
    insertion_mode: InsertionMode,
    stack_of_open_elements: StackOfOpenElements,
    stack_of_template_insertion_modes: Vec<InsertionMode>,
}
impl ProcessorState {
    fn new() -> Self {
        Self {
            active_formatting_elements: ActiveFormattingElements::new(),
            current_token: None,
            encoding: "UTF-8".into(),
            encoding_confidence: EncodingConfidence::Tentative,
            form_element: None,
            frameset_ok: true,
            head_element: None,
            insertion_mode: InsertionMode::INITIAL,
            stack_of_open_elements: StackOfOpenElements::new(),
            stack_of_template_insertion_modes: Vec::new(),
        }
    }
}
enum EncodingConfidence {
    Tentative,
    Certain,
    Irrelevant,
}
/// Insertion mode.
///
/// @see https://html.spec.whatwg.org/#the-insertion-mode
#[derive(Debug)]
enum InsertionMode {
    /// Initial insertion mode for full HTML parser.
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#the-initial-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    INITIAL,

    /// Before HTML insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#the-before-html-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    BEFORE_HTML,

    /// Before head insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-beforehead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    BEFORE_HEAD,

    /// In head insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inhead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_HEAD,

    /// In head noscript insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inheadnoscript
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_HEAD_NOSCRIPT,

    /// After head insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterhead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_HEAD,

    /// In body insertion mode for full HTML parser.
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inbody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_BODY,

    /// In table insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intable
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE,

    /// In table text insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intabletext
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE_TEXT,

    /// In caption insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incaption
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_CAPTION,

    /// In column group insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incolumngroup
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_COLUMN_GROUP,

    /// In table body insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intablebody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE_BODY,

    /// In row insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inrow
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_ROW,

    /// In cell insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incell
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_CELL,

    /// In select insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselect
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_SELECT,

    /// In select in table insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselectintable
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_SELECT_IN_TABLE,

    /// In template insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intemplate
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TEMPLATE,

    /// After body insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterbody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_BODY,

    /// In frameset insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inframeset
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_FRAMESET,

    /// After frameset insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterframeset
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_FRAMESET,

    /// After after body insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-body-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_AFTER_BODY,

    /// After after frameset insertion mode for full HTML parser.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-frameset-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_AFTER_FRAMESET,
}

struct HTMLStackEvent {
    operation: StackOperation,
    token: HTMLToken,
    provenance: StackProvenance,
}

#[derive(PartialEq)]
enum StackOperation {
    Push,
    Pop,
}
#[derive(PartialEq)]
enum StackProvenance {
    Real,
    Virtual,
}

pub struct HtmlProcessor {
    tag_processor: TagProcessor,
    state: ProcessorState,
    last_error: Option<String>,
    unsupported_exception: Option<String>,
    element_queue: Vec<HTMLStackEvent>,
    current_element: Option<HTMLStackEvent>,
}

impl HtmlProcessor {
    /// Creates an HTML processor in the fragment parsing mode.
    ///
    /// Use this for cases where you are processing chunks of HTML that
    /// will be found within a bigger HTML document, such as rendered
    /// block output that exists within a post, `the_content` inside a
    /// rendered site layout.
    ///
    /// Fragment parsing occurs within a context, which is an HTML element
    /// that the document will eventually be placed in. It becomes important
    /// when special elements have different rules than others, such as inside
    /// a TEXTAREA or a TITLE tag where things that look like tags are text,
    /// or inside a SCRIPT tag where things that look like HTML syntax are JS.
    ///
    /// The context value should be a representation of the tag into which the
    /// HTML is found. For most cases this will be the body element. The HTML
    /// form is provided because a context element may have attributes that
    /// impact the parse, such as with a SCRIPT tag and its `type` attribute.
    ///
    /// ## Current HTML Support
    ///
    ///  - The only supported context is `<body>`, which is the default value.
    ///  - The only supported document encoding is `UTF-8`, which is the default value.
    ///
    /// @param string $html     Input HTML fragment to process.
    /// @param string $context  Context element for the fragment, must be default of `<body>`.
    /// @param string $encoding Text encoding of the document; must be default of 'UTF-8'.
    /// @return static|null The created processor if successful, otherwise null.
    pub fn create_fragment(html: &str, known_definite_encoding: &str) -> Option<Self> {
        todo!()
    }

    /// Creates an HTML processor in the full parsing mode.
    ///
    /// It's likely that a fragment parser is more appropriate, unless sending an
    /// entire HTML document from start to finish. Consider a fragment parser with
    /// a context node of `<body>`.
    ///
    /// UTF-8 is the only allowed encoding. If working with a document that
    /// isn't UTF-8, first convert the document to UTF-8, then pass in the
    /// converted HTML.
    ///
    /// @param string      $html                    Input HTML document to process.
    /// @param string|null $known_definite_encoding Optional. If provided, specifies the charset used
    ///                                             in the input byte stream. Currently must be UTF-8.
    /// @return static|null The created processor if successful, otherwise null.
    pub fn create_full_parser(html: &str, known_definite_encoding: &str) -> Option<Self> {
        if "UTF-8" != known_definite_encoding {
            return None;
        }

        let mut processor = Self::new(html);
        processor.state.encoding = "UTF-8".into();
        processor.state.encoding_confidence = EncodingConfidence::Certain;

        Some(processor)
    }

    fn new(html: &str) -> Self {
        let tag_processor = TagProcessor::new(html);
        let state = ProcessorState::new();

        // TODO stack push/pop handlers???

        Self {
            tag_processor,
            state,
            element_queue: Vec::new(),
            last_error: None,
            unsupported_exception: None,
            current_element: None,
        }
    }

    /// Creates a fragment processor at the current node.
    ///
    /// HTML Fragment parsing always happens with a context node. HTML Fragment Processors can be
    /// instantiated with a `BODY` context node via `WP_HTML_Processor::create_fragment( $html )`.
    ///
    /// The context node may impact how a fragment of HTML is parsed. For example, consider the HTML
    /// fragment `<td />Inside TD?</td>`.
    ///
    /// A BODY context node will produce the following tree:
    ///
    ///     └─#text Inside TD?
    ///
    /// Notice that the `<td>` tags are completely ignored.
    ///
    /// Compare that with an SVG context node that produces the following tree:
    ///
    ///     ├─svg:td
    ///     └─#text Inside TD?
    ///
    /// Here, a `td` node in the `svg` namespace is created, and its self-closing flag is respected.
    /// This is a peculiarity of parsing HTML in foreign content like SVG.
    ///
    /// Finally, consider the tree produced with a TABLE context node:
    ///
    ///     └─TBODY
    ///       └─TR
    ///         └─TD
    ///           └─#text Inside TD?
    ///
    /// These examples demonstrate how important the context node may be when processing an HTML
    /// fragment. Special care must be taken when processing fragments that are expected to appear
    /// in specific contexts. SVG and TABLE are good examples, but there are others.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#html-fragment-parsing-algorithm
    ///
    /// @since 6.8.0
    ///
    /// @param string $html Input HTML fragment to process.
    /// @return static|null The created processor if successful, otherwise null.
    fn create_fragment_at_current_node(html: &str) -> Self {
        todo!()
    }

    /// Stops the parser and terminates its execution when encountering unsupported markup.
    ///
    /// @throws WP_HTML_Unsupported_Exception Halts execution of the parser.
    ///
    /// @since 6.7.0
    ///
    /// @param string $message Explains support is missing in order to parse the current node.
    fn bail(&mut self, message: String) -> () {
        todo!()
    }

    /// Returns the last error, if any.
    ///
    /// Various situations lead to parsing failure but this class will
    /// return `false` in all those cases. To determine why something
    /// failed it's possible to request the last error. This can be
    /// helpful to know to distinguish whether a given tag couldn't
    /// be found or if content in the document caused the processor
    /// to give up and abort processing.
    ///
    /// Example
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<template><strong><button><em><p><em>' );
    ///     false === $processor->next_tag();
    ///     WP_HTML_Processor::ERROR_UNSUPPORTED === $processor->get_last_error();
    ///
    /// @since 6.4.0
    ///
    /// @see self::ERROR_UNSUPPORTED
    /// @see self::ERROR_EXCEEDED_MAX_BOOKMARKS
    ///
    /// @return string|null The last error, if one exists, otherwise null.
    pub fn get_last_error(&self) -> &Option<String> {
        &self.last_error
    }

    /// Returns context for why the parser aborted due to unsupported HTML, if it did.
    ///
    /// This is meant for debugging purposes, not for production use.
    ///
    /// @since 6.7.0
    ///
    /// @see self::$unsupported_exception
    ///
    /// @return WP_HTML_Unsupported_Exception|null

    pub fn get_unsupported_exception(&self) -> &Option<String> {
        &self.unsupported_exception
    }

    /// Finds the next tag matching the $query.
    ///
    /// @todo Support matching the class name and tag name.
    ///
    /// @since 6.4.0
    /// @since 6.6.0 Visits all tokens, including virtual ones.
    ///
    /// @throws Exception When unable to allocate a bookmark for the next token in the input HTML document.
    ///
    /// @param array|string|null $query {
    ///     Optional. Which tag name to find, having which class, etc. Default is to find any tag.
    ///
    ///     @type string|null $tag_name     Which tag to find, or `null` for "any tag."
    ///     @type string      $tag_closers  'visit' to pause at tag closers, 'skip' or unset to only visit openers.
    ///     @type int|null    $match_offset Find the Nth tag matching all search criteria.
    ///                                     1 for "first" tag, 3 for "third," etc.
    ///                                     Defaults to first tag.
    ///     @type string|null $class_name   Tag must contain this whole class name to match.
    ///     @type string[]    $breadcrumbs  DOM sub-path at which element is found, e.g. `array( 'FIGURE', 'IMG' )`.
    ///                                     May also contain the wildcard `*` which matches a single element, e.g. `array( 'SECTION', '*' )`.
    /// }
    /// @return bool Whether a tag was matched.
    pub fn next_tag(&mut self, query: ()) -> bool {
        todo!()
    }

    /// Finds the next token in the HTML document.
    ///
    /// This doesn't currently have a way to represent non-tags and doesn't process
    /// semantic rules for text nodes. For access to the raw tokens consider using
    /// WP_HTML_Tag_Processor instead.
    ///
    /// @since 6.5.0 Added for internal support; do not use.
    /// @since 6.7.1 Refactored so subclasses may extend.
    ///
    /// @return bool Whether a token was parsed.

    pub fn next_token(&mut self) -> bool {
        self.next_visitable_token()
    }

    /// Ensures internal accounting is maintained for HTML semantic rules while
    /// the underlying Tag Processor class is seeking to a bookmark.
    ///
    /// This doesn't currently have a way to represent non-tags and doesn't process
    /// semantic rules for text nodes. For access to the raw tokens consider using
    /// WP_HTML_Tag_Processor instead.
    ///
    /// Note that this method may call itself recursively. This is why it is not
    /// implemented as {@see WP_HTML_Processor::next_token()}, which instead calls
    /// this method similarly to how {@see WP_HTML_Tag_Processor::next_token()}
    /// calls the {@see WP_HTML_Tag_Processor::base_class_next_token()} method.
    ///
    /// @since 6.7.1 Added for internal support.
    ///
    /// @access private
    ///
    /// @return bool

    fn next_visitable_token(&mut self) -> bool {
        todo!()
    }

    /// Indicates if the current tag token is a tag closer.
    ///
    /// Example:
    ///
    ///     $p = WP_HTML_Processor::create_fragment( '<div></div>' );
    ///     $p->next_tag( array( 'tag_name' => 'div', 'tag_closers' => 'visit' ) );
    ///     $p->is_tag_closer() === false;
    ///
    ///     $p->next_tag( array( 'tag_name' => 'div', 'tag_closers' => 'visit' ) );
    ///     $p->is_tag_closer() === true;
    ///
    /// @since 6.6.0 Subclassed for HTML Processor.
    ///
    /// @return bool Whether the current tag is a tag closer.

    pub fn is_tag_closer(&self) -> bool {
        if self.is_virtual() {
            self.current_element
                .as_ref()
                .expect("Must have current element if is virtual")
                .operation
                == StackOperation::Pop
                && self
                    .get_token_type()
                    .map(|t| t == TokenType::Tag)
                    .unwrap_or(false)
        } else {
            self.tag_processor.is_tag_closer()
        }
    }

    /// Indicates if the currently-matched token is virtual, created by a stack operation
    /// while processing HTML, rather than a token found in the HTML text itself.
    ///
    /// @since 6.6.0
    ///
    /// @return bool Whether the current token is virtual.

    fn is_virtual(&self) -> bool {
        self.current_element
            .as_ref()
            .map(|current_element| current_element.provenance == StackProvenance::Virtual)
            .unwrap_or(false)
    }

    /// Indicates if the currently-matched tag matches the given breadcrumbs.
    ///
    /// A "*" represents a single tag wildcard, where any tag matches, but not no tags.
    ///
    /// At some point this function _may_ support a `**` syntax for matching any number
    /// of unspecified tags in the breadcrumb stack. This has been intentionally left
    /// out, however, to keep this function simple and to avoid introducing backtracking,
    /// which could open up surprising performance breakdowns.
    ///
    /// Example:
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<div><span><figure><img></figure></span></div>' );
    ///     $processor->next_tag( 'img' );
    ///     true  === $processor->matches_breadcrumbs( array( 'figure', 'img' ) );
    ///     true  === $processor->matches_breadcrumbs( array( 'span', 'figure', 'img' ) );
    ///     false === $processor->matches_breadcrumbs( array( 'span', 'img' ) );
    ///     true  === $processor->matches_breadcrumbs( array( 'span', '*', 'img' ) );
    ///
    /// @since 6.4.0
    ///
    /// @param string[] $breadcrumbs DOM sub-path at which element is found, e.g. `array( 'FIGURE', 'IMG' )`.
    ///                              May also contain the wildcard `*` which matches a single element, e.g. `array( 'SECTION', '*' )`.
    /// @return bool Whether the currently-matched tag is found at the given nested structure.

    pub fn matches_breadcrumbs(breadcrumbs: ()) -> bool {
        todo!()
    }

    /// Indicates if the currently-matched node expects a closing
    /// token, or if it will self-close on the next step.
    ///
    /// Most HTML elements expect a closer, such as a P element or
    /// a DIV element. Others, like an IMG element are void and don't
    /// have a closing tag. Special elements, such as SCRIPT and STYLE,
    /// are treated just like void tags. Text nodes and self-closing
    /// foreign content will also act just like a void tag, immediately
    /// closing as soon as the processor advances to the next token.
    ///
    /// @since 6.6.0
    ///
    /// @param WP_HTML_Token|null $node Optional. Node to examine, if provided.
    ///                                 Default is to examine current node.
    /// @return bool|null Whether to expect a closer for the currently-matched node,
    ///                   or `null` if not matched on any token.

    pub fn expects_closer(node: Option<HTMLToken>) -> bool {
        todo!()
    }

    /// Steps through the HTML document and stop at the next tag, if any.
    ///
    /// @since 6.4.0
    ///
    /// @throws Exception When unable to allocate a bookmark for the next token in the input HTML document.
    ///
    /// @see self::PROCESS_NEXT_NODE
    /// @see self::REPROCESS_CURRENT_NODE
    ///
    /// @param string $node_to_process Whether to parse the next node or reprocess the current node.
    /// @return bool Whether a tag was matched.

    pub fn step(node_to_process: NodeToProcess) -> bool {
        todo!()
    }

    /// Computes the HTML breadcrumbs for the currently-matched node, if matched.
    ///
    /// Breadcrumbs start at the outermost parent and descend toward the matched element.
    /// They always include the entire path from the root HTML node to the matched element.
    ///
    /// Example:
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<p><strong><em><img></em></strong></p>' );
    ///     $processor->next_tag( 'IMG' );
    ///     $processor->get_breadcrumbs() === array( 'HTML', 'BODY', 'P', 'STRONG', 'EM', 'IMG' );
    ///
    /// @since 6.4.0
    ///
    /// @return string[] Array of tag names representing path to matched node.

    pub fn get_breadcrumbs() -> () {
        todo!()
    }

    /// Returns the nesting depth of the current location in the document.
    ///
    /// Example:
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<div><p></p></div>' );
    ///     // The processor starts in the BODY context, meaning it has depth from the start: HTML > BODY.
    ///     2 === $processor->get_current_depth();
    ///
    ///     // Opening the DIV element increases the depth.
    ///     $processor->next_token();
    ///     3 === $processor->get_current_depth();
    ///
    ///     // Opening the P element increases the depth.
    ///     $processor->next_token();
    ///     4 === $processor->get_current_depth();
    ///
    ///     // The P element is closed during `next_token()` so the depth is decreased to reflect that.
    ///     $processor->next_token();
    ///     3 === $processor->get_current_depth();
    ///
    /// @since 6.6.0
    ///
    /// @return int Nesting-depth of current location in the document.

    pub fn get_current_depth() -> usize {
        todo!()
    }

    /// Normalizes an HTML fragment by serializing it.
    ///
    /// This method assumes that the given HTML snippet is found in BODY context.
    /// For normalizing full documents or fragments found in other contexts, create
    /// a new processor using {@see WP_HTML_Processor::create_fragment} or
    /// {@see WP_HTML_Processor::create_full_parser} and call {@see WP_HTML_Processor::serialize}
    /// on the created instances.
    ///
    /// Many aspects of an input HTML fragment may be changed during normalization.
    ///
    ///  - Attribute values will be double-quoted.
    ///  - Duplicate attributes will be removed.
    ///  - Omitted tags will be added.
    ///  - Tag and attribute name casing will be lower-cased,
    ///    except for specific SVG and MathML tags or attributes.
    ///  - Text will be re-encoded, null bytes handled,
    ///    and invalid UTF-8 replaced with U+FFFD.
    ///  - Any incomplete syntax trailing at the end will be omitted,
    ///    for example, an unclosed comment opener will be removed.
    ///
    /// Example:
    ///
    ///     echo WP_HTML_Processor::normalize( '<a href=#anchor v=5 href="/" enabled>One</a another v=5><!--' );
    ///     // <a href="#anchor" v="5" enabled>One</a>
    ///
    ///     echo WP_HTML_Processor::normalize( '<div></p>fun<table><td>cell</div>' );
    ///     // <div><p></p>fun<table><tbody><tr><td>cell</td></tr></tbody></table></div>
    ///
    ///     echo WP_HTML_Processor::normalize( '<![CDATA[invalid comment]]> syntax < <> "oddities"' );
    ///     // <!--[CDATA[invalid comment]]--> syntax &lt; &lt;&gt; &quot;oddities&quot;
    ///
    /// @since 6.7.0
    ///
    /// @param string $html Input HTML to normalize.
    ///
    /// @return string|null Normalized output, or `null` if unable to normalize.

    pub fn normalize(html: &str) -> Result<String, ()> {
        let processor = Self::create_fragment(html, "UTF-8")
            .expect("Fragment creation fails when not UTF-8. Statically set here.");
        processor.serialize()
    }

    /// Returns normalized HTML for a fragment by serializing it.
    ///
    /// This differs from {@see WP_HTML_Processor::normalize} in that it starts with
    /// a specific HTML Processor, which _must_ not have already started scanning;
    /// it must be in the initial ready state and will be in the completed state once
    /// serialization is complete.
    ///
    /// Many aspects of an input HTML fragment may be changed during normalization.
    ///
    ///  - Attribute values will be double-quoted.
    ///  - Duplicate attributes will be removed.
    ///  - Omitted tags will be added.
    ///  - Tag and attribute name casing will be lower-cased,
    ///    except for specific SVG and MathML tags or attributes.
    ///  - Text will be re-encoded, null bytes handled,
    ///    and invalid UTF-8 replaced with U+FFFD.
    ///  - Any incomplete syntax trailing at the end will be omitted,
    ///    for example, an unclosed comment opener will be removed.
    ///
    /// Example:
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<a href=#anchor v=5 href="/" enabled>One</a another v=5><!--' );
    ///     echo $processor->serialize();
    ///     // <a href="#anchor" v="5" enabled>One</a>
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<div></p>fun<table><td>cell</div>' );
    ///     echo $processor->serialize();
    ///     // <div><p></p>fun<table><tbody><tr><td>cell</td></tr></tbody></table></div>
    ///
    ///     $processor = WP_HTML_Processor::create_fragment( '<![CDATA[invalid comment]]> syntax < <> "oddities"' );
    ///     echo $processor->serialize();
    ///     // <!--[CDATA[invalid comment]]--> syntax &lt; &lt;&gt; &quot;oddities&quot;
    ///
    /// @since 6.7.0
    ///
    /// @return string|null Normalized HTML markup represented by processor,
    ///                     or `null` if unable to generate serialization.
    pub fn serialize(&self) -> Result<String, ()> {
        todo!()
    }

    /// Serializes the currently-matched token.
    ///
    /// This method produces a fully-normative HTML string for the currently-matched token,
    /// if able. If not matched at any token or if the token doesn't correspond to any HTML
    /// it will return an empty string (for example, presumptuous end tags are ignored).
    ///
    /// @see static::serialize()
    ///
    /// @since 6.7.0
    ///
    /// @return string Serialization of token, or empty string if no serialization exists.
    ///
    /// @todo What do wo with this _protected_ function?
    fn serialize_token() -> String {
        todo!()
    }

    /// Parses next element in the 'initial' insertion mode.
    ///
    /// This internal function performs the 'initial' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-initial-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_initial(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'before html' insertion mode.
    ///
    /// This internal function performs the 'before html' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-before-html-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_before_html(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'before head' insertion mode.
    ///
    /// This internal function performs the 'before head' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-before-head-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_before_head(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in head' insertion mode.
    ///
    /// This internal function performs the 'in head' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_head(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in head noscript' insertion mode.
    ///
    /// This internal function performs the 'in head noscript' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inheadnoscript
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_head_noscript(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'after head' insertion mode.
    ///
    /// This internal function performs the 'after head' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-head-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_after_head(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in body' insertion mode.
    ///
    /// This internal function performs the 'in body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.4.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inbody
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_body(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in table' insertion mode.
    ///
    /// This internal function performs the 'in table' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intable
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_table(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in table text' insertion mode.
    ///
    /// This internal function performs the 'in table text' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intabletext
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_table_text(&mut self) -> () {
        todo!("should become a result");
        self.bail(format!(
            "No support for parsing in the {:?} state.",
            InsertionMode::IN_TABLE_TEXT
        ))
    }

    /// Parses next element in the 'in caption' insertion mode.
    ///
    /// This internal function performs the 'in caption' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incaption
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_caption(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in column group' insertion mode.
    ///
    /// This internal function performs the 'in column group' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incolgroup
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_column_group(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in table body' insertion mode.
    ///
    /// This internal function performs the 'in table body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intbody
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_table_body(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in row' insertion mode.
    ///
    /// This internal function performs the 'in row' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intr
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_row(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in cell' insertion mode.
    ///
    /// This internal function performs the 'in cell' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intd
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_cell(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in select' insertion mode.
    ///
    /// This internal function performs the 'in select' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inselect
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_select(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in select in table' insertion mode.
    ///
    /// This internal function performs the 'in select in table' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselectintable
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_select_in_table(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in template' insertion mode.
    ///
    /// This internal function performs the 'in template' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intemplate
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_template(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'after body' insertion mode.
    ///
    /// This internal function performs the 'after body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterbody
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_after_body(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in frameset' insertion mode.
    ///
    /// This internal function performs the 'in frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inframeset
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_frameset(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'after frameset' insertion mode.
    ///
    /// This internal function performs the 'after frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterframeset
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_after_frameset(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'after after body' insertion mode.
    ///
    /// This internal function performs the 'after after body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-body-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_after_after_body(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'after after frameset' insertion mode.
    ///
    /// This internal function performs the 'after after frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-frameset-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_after_after_frameset(&mut self) -> bool {
        todo!()
    }

    /// Parses next element in the 'in foreign content' insertion mode.
    ///
    /// This internal function performs the 'in foreign content' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @since 6.7.0 Stub implementation.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inforeign
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.

    fn step_in_foreign_content(&mut self) -> bool {
        todo!()
    }

    /// Internal helpers

    /// Creates a new bookmark for the currently-matched token and returns the generated name.
    ///
    /// @since 6.4.0
    /// @since 6.5.0 Renamed from bookmark_tag() to bookmark_token().
    ///
    /// @throws Exception When unable to allocate requested bookmark.
    ///
    /// @return string|false Name of created bookmark, or false if unable to create.

    fn bookmark_token(&mut self) {
        todo!()
    }

    /// HTML semantic overrides for Tag Processor

    /// Indicates the namespace of the current token, or "html" if there is none.
    ///
    /// @return string One of "html", "math", or "svg".

    pub fn get_namespace(&self) -> ParsingNamespace {
        todo!()
    }

    /// Returns the uppercase name of the matched tag.
    ///
    /// The semantic rules for HTML specify that certain tags be reprocessed
    /// with a different tag name. Because of this, the tag name presented
    /// by the HTML Processor may differ from the one reported by the HTML
    /// Tag Processor, which doesn't apply these semantic rules.
    ///
    /// Example:
    ///
    ///     $processor = new WP_HTML_Tag_Processor( '<div class="test">Test</div>' );
    ///     $processor->next_tag() === true;
    ///     $processor->get_tag() === 'DIV';
    ///
    ///     $processor->next_tag() === false;
    ///     $processor->get_tag() === null;
    ///
    /// @since 6.4.0
    ///
    /// @return string|null Name of currently matched tag in input HTML, or `null` if none found.

    pub fn get_tag(&self) -> Option<String> {
        todo!()
    }

    /// Indicates if the currently matched tag contains the self-closing flag.
    ///
    /// No HTML elements ought to have the self-closing flag and for those, the self-closing
    /// flag will be ignored. For void elements this is benign because they "self close"
    /// automatically. For non-void HTML elements though problems will appear if someone
    /// intends to use a self-closing element in place of that element with an empty body.
    /// For HTML foreign elements and custom elements the self-closing flag determines if
    /// they self-close or not.
    ///
    /// This function does not determine if a tag is self-closing,
    /// but only if the self-closing flag is present in the syntax.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @return bool Whether the currently matched tag contains the self-closing flag.

    pub fn has_self_closing_flag(&self) -> bool {
        if self.is_virtual() {
            false
        } else {
            self.tag_processor.has_self_closing_flag()
        }
    }

    /// Returns the node name represented by the token.
    ///
    /// This matches the DOM API value `nodeName`. Some values
    /// are static, such as `#text` for a text node, while others
    /// are dynamically generated from the token itself.
    ///
    /// Dynamic names:
    ///  - Uppercase tag name for tag matches.
    ///  - `html` for DOCTYPE declarations.
    ///
    /// Note that if the Tag Processor is not matched on a token
    /// then this function will return `null`, either because it
    /// hasn't yet found a token or because it reached the end
    /// of the document without matching a token.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @return string|null Name of the matched token.

    pub fn get_token_name(&self) {
        todo!()
    }

    /// Indicates the kind of matched token, if any.
    ///
    /// This differs from `get_token_name()` in that it always
    /// returns a static string indicating the type, whereas
    /// `get_token_name()` may return values derived from the
    /// token itself, such as a tag name or processing
    /// instruction tag.
    ///
    /// Possible values:
    ///  - `#tag` when matched on a tag.
    ///  - `#text` when matched on a text node.
    ///  - `#cdata-section` when matched on a CDATA node.
    ///  - `#comment` when matched on a comment.
    ///  - `#doctype` when matched on a DOCTYPE declaration.
    ///  - `#presumptuous-tag` when matched on an empty tag closer.
    ///  - `#funky-comment` when matched on a funky comment.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @return string|null What kind of token is matched, or null.

    pub fn get_token_type(&self) -> Option<TokenType> {
        if self.is_virtual() {
            /*
             * This logic comes from the Tag Processor.
             *
             * @todo It would be ideal not to repeat this here, but it's not clearly
             *       better to allow passing a token name to `get_token_type()`.
             */
            let node_name = &self.current_element.as_ref().unwrap().token.node_name;
            let starting_char = node_name.as_bytes()[0];
            if b'A' <= starting_char && b'Z' >= starting_char {
                Some(TokenType::Tag)
            } else if node_name.as_ref() == "html" {
                Some(TokenType::Doctype)
            } else {
                todo!("Implement other token types")
            }
        } else {
            self.tag_processor.get_token_type()
        }
    }

    /// Returns the value of a requested attribute from a matched tag opener if that attribute exists.
    ///
    /// Example:
    ///
    ///     $p = WP_HTML_Processor::create_fragment( '<div enabled class="test" data-test-id="14">Test</div>' );
    ///     $p->next_token() === true;
    ///     $p->get_attribute( 'data-test-id' ) === '14';
    ///     $p->get_attribute( 'enabled' ) === true;
    ///     $p->get_attribute( 'aria-label' ) === null;
    ///
    ///     $p->next_tag() === false;
    ///     $p->get_attribute( 'class' ) === null;
    ///
    /// @since 6.6.0 Subclassed for HTML Processor.
    ///
    /// @param string $name Name of attribute whose value is requested.
    /// @return string|true|null Value of attribute or `null` if not available. Boolean attributes return `true`.

    pub fn get_attribute(&self, name: &str) -> Option<String> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.get_attribute(name)
        }
    }

    /// Updates or creates a new attribute on the currently matched tag with the passed value.
    ///
    /// For boolean attributes special handling is provided:
    ///  - When `true` is passed as the value, then only the attribute name is added to the tag.
    ///  - When `false` is passed, the attribute gets removed if it existed before.
    ///
    /// For string attributes, the value is escaped using the `esc_attr` function.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @param string      $name  The attribute name to target.
    /// @param string|bool $value The new attribute value.
    /// @return bool Whether an attribute value was set.

    pub fn set_attribute(&mut self, name: &str, value: &str) -> bool {
        if self.is_virtual() {
            false
        } else {
            self.tag_processor.set_attribute(name, value)
        }
    }

    /// Remove an attribute from the currently-matched tag.
    ///
    /// @since 6.6.0 Subclassed for HTML Processor.
    ///
    /// @param string $name The attribute name to remove.
    /// @return bool Whether an attribute was removed.

    pub fn remove_attribute(&mut self, name: &str) -> bool {
        if self.is_virtual() {
            false
        } else {
            self.tag_processor.remove_attribute(name)
        }
    }

    /// Gets lowercase names of all attributes matching a given prefix in the current tag.
    ///
    /// Note that matching is case-insensitive. This is in accordance with the spec:
    ///
    /// > There must never be two or more attributes on
    /// > the same start tag whose names are an ASCII
    /// > case-insensitive match for each other.
    ///     - HTML 5 spec
    ///
    /// Example:
    ///
    ///     $p = new WP_HTML_Tag_Processor( '<div data-ENABLED class="test" DATA-test-id="14">Test</div>' );
    ///     $p->next_tag( array( 'class_name' => 'test' ) ) === true;
    ///     $p->get_attribute_names_with_prefix( 'data-' ) === array( 'data-enabled', 'data-test-id' );
    ///
    ///     $p->next_tag() === false;
    ///     $p->get_attribute_names_with_prefix( 'data-' ) === null;
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @see https://html.spec.whatwg.org/multipage/syntax.html#attributes-2:ascii-case-insensitive
    ///
    /// @param string $prefix Prefix of requested attribute names.
    /// @return array|null List of attribute names, or `null` when no tag opener is matched.

    pub fn get_attribute_names_with_prefix(&self, prefix: &str) -> Option<Vec<Box<str>>> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.get_attribute_names_with_prefix(prefix)
        }
    }

    /// Adds a new class name to the currently matched tag.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @param string $class_name The class name to add.
    /// @return bool Whether the class was set to be added.

    pub fn add_class(&mut self, class_name: &str) -> bool {
        if self.is_virtual() {
            false
        } else {
            self.tag_processor.add_class(class_name)
        }
    }

    /// Removes a class name from the currently matched tag.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @param string $class_name The class name to remove.
    /// @return bool Whether the class was set to be removed.

    pub fn remove_class(&mut self, class_name: &str) -> bool {
        if self.is_virtual() {
            false
        } else {
            self.tag_processor.remove_class(class_name)
        }
    }

    /// Returns if a matched tag contains the given ASCII case-insensitive class name.
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @todo When reconstructing active formatting elements with attributes, find a way
    ///       to indicate if the virtually-reconstructed formatting elements contain the
    ///       wanted class name.
    ///
    /// @param string $wanted_class Look for this CSS class name, ASCII case-insensitive.
    /// @return bool|null Whether the matched tag contains the given class name, or null if not matched.

    pub fn has_class(&self, wanted_class: &str) -> Option<bool> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.has_class(wanted_class)
        }
    }

    /// Generator for a foreach loop to step through each class name for the matched tag.
    ///
    /// This generator function is designed to be used inside a "foreach" loop.
    ///
    /// Example:
    ///
    ///     $p = WP_HTML_Processor::create_fragment( "<div class='free &lt;egg&lt;\tlang-en'>" );
    ///     $p->next_tag();
    ///     foreach ( $p->class_list() as $class_name ) {
    ///         echo "{$class_name} ";
    ///     }
    ///     // Outputs: "free <egg> lang-en "
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.

    pub fn class_list(&self) -> () {
        todo!();
        //if self.is_virtual() {
        //    None
        //} else {
        //    self.tag_processor.class_list()
        //}
    }

    /// Returns the modifiable text for a matched token, or an empty string.
    ///
    /// Modifiable text is text content that may be read and changed without
    /// changing the HTML structure of the document around it. This includes
    /// the contents of `#text` nodes in the HTML as well as the inner
    /// contents of HTML comments, Processing Instructions, and others, even
    /// though these nodes aren't part of a parsed DOM tree. They also contain
    /// the contents of SCRIPT and STYLE tags, of TEXTAREA tags, and of any
    /// other section in an HTML document which cannot contain HTML markup (DATA).
    ///
    /// If a token has no modifiable text then an empty string is returned to
    /// avoid needless crashing or type errors. An empty string does not mean
    /// that a token has modifiable text, and a token with modifiable text may
    /// have an empty string (e.g. a comment with no contents).
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @return string
    pub fn get_modifiable_text(&self) -> Box<str> {
        if self.is_virtual() {
            "".into()
        } else {
            self.tag_processor.get_modifiable_text()
        }
    }

    /// Indicates what kind of comment produced the comment node.
    ///
    /// Because there are different kinds of HTML syntax which produce
    /// comments, the Tag Processor tracks and exposes this as a type
    /// for the comment. Nominally only regular HTML comments exist as
    /// they are commonly known, but a number of unrelated syntax errors
    /// also produce comments.
    ///
    /// @see self::COMMENT_AS_ABRUPTLY_CLOSED_COMMENT
    /// @see self::COMMENT_AS_CDATA_LOOKALIKE
    /// @see self::COMMENT_AS_INVALID_HTML
    /// @see self::COMMENT_AS_HTML_COMMENT
    /// @see self::COMMENT_AS_PI_NODE_LOOKALIKE
    ///
    /// @since 6.6.0 Subclassed for the HTML Processor.
    ///
    /// @return string|null
    pub fn get_comment_type(&self) -> Option<CommentType> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.get_comment_type()
        }
    }

    /// Removes a bookmark that is no longer needed.
    ///
    /// Releasing a bookmark frees up the small
    /// performance overhead it requires.
    ///
    /// @since 6.4.0
    ///
    /// @param string $bookmark_name Name of the bookmark to remove.
    /// @return bool Whether the bookmark already existed before removal.
    pub fn release_bookmark(&mut self, bookmark_name: &str) -> bool {
        todo!()
    }

    /// Moves the internal cursor in the HTML Processor to a given bookmark's location.
    ///
    /// Be careful! Seeking backwards to a previous location resets the parser to the
    /// start of the document and reparses the entire contents up until it finds the
    /// sought-after bookmarked location.
    ///
    /// In order to prevent accidental infinite loops, there's a
    /// maximum limit on the number of times seek() can be called.
    ///
    /// @throws Exception When unable to allocate a bookmark for the next token in the input HTML document.
    ///
    /// @since 6.4.0
    ///
    /// @param string $bookmark_name Jump to the place in the document identified by this bookmark name.
    /// @return bool Whether the internal cursor was successfully moved to the bookmark's location.
    pub fn seek(&mut self, bookmark_name: &str) -> bool {
        todo!()
    }

    /// Sets a bookmark in the HTML document.
    ///
    /// Bookmarks represent specific places or tokens in the HTML
    /// document, such as a tag opener or closer. When applying
    /// edits to a document, such as setting an attribute, the
    /// text offsets of that token may shift; the bookmark is
    /// kept updated with those shifts and remains stable unless
    /// the entire span of text in which the token sits is removed.
    ///
    /// Release bookmarks when they are no longer needed.
    ///
    /// Example:
    ///
    ///     <main><h2>Surprising fact you may not know!</h2></main>
    ///           ^  ^
    ///            \-|-- this `H2` opener bookmark tracks the token
    ///
    ///     <main class="clickbait"><h2>Surprising fact you may no…
    ///                             ^  ^
    ///                              \-|-- it shifts with edits
    ///
    /// Bookmarks provide the ability to seek to a previously-scanned
    /// place in the HTML document. This avoids the need to re-scan
    /// the entire document.
    ///
    /// Example:
    ///
    ///     <ul><li>One</li><li>Two</li><li>Three</li></ul>
    ///                                 ^^^^
    ///                                 want to note this last item
    ///
    ///     $p = new WP_HTML_Tag_Processor( $html );
    ///     $in_list = false;
    ///     while ( $p->next_tag( array( 'tag_closers' => $in_list ? 'visit' : 'skip' ) ) ) {
    ///         if ( 'UL' === $p->get_tag() ) {
    ///             if ( $p->is_tag_closer() ) {
    ///                 $in_list = false;
    ///                 $p->set_bookmark( 'resume' );
    ///                 if ( $p->seek( 'last-li' ) ) {
    ///                     $p->add_class( 'last-li' );
    ///                 }
    ///                 $p->seek( 'resume' );
    ///                 $p->release_bookmark( 'last-li' );
    ///                 $p->release_bookmark( 'resume' );
    ///             } else {
    ///                 $in_list = true;
    ///             }
    ///         }
    ///
    ///         if ( 'LI' === $p->get_tag() ) {
    ///             $p->set_bookmark( 'last-li' );
    ///         }
    ///     }
    ///
    /// Bookmarks intentionally hide the internal string offsets
    /// to which they refer. They are maintained internally as
    /// updates are applied to the HTML document and therefore
    /// retain their "position" - the location to which they
    /// originally pointed. The inability to use bookmarks with
    /// functions like `substr` is therefore intentional to guard
    /// against accidentally breaking the HTML.
    ///
    /// Because bookmarks allocate memory and require processing
    /// for every applied update, they are limited and require
    /// a name. They should not be created with programmatically-made
    /// names, such as "li_{$index}" with some loop. As a general
    /// rule they should only be created with string-literal names
    /// like "start-of-section" or "last-paragraph".
    ///
    /// Bookmarks are a powerful tool to enable complicated behavior.
    /// Consider double-checking that you need this tool if you are
    /// reaching for it, as inappropriate use could lead to broken
    /// HTML structure or unwanted processing overhead.
    ///
    /// Bookmarks cannot be set on tokens that do no appear in the original
    /// HTML text. For example, the HTML `<table><td>` stops at tags `TABLE`,
    /// `TBODY`, `TR`, and `TD`. The `TBODY` and `TR` tags do not appear in
    /// the original HTML and cannot be used as bookmarks.
    ///
    /// @since 6.4.0
    ///
    /// @param string $bookmark_name Identifies this particular bookmark.
    /// @return bool Whether the bookmark was successfully created.
    pub fn set_bookmark(&mut self, bookmark_name: &str) -> bool {
        todo!()
    }

    /// Checks whether a bookmark with the given name exists.
    ///
    /// @since 6.5.0
    ///
    /// @param string $bookmark_name Name to identify a bookmark that potentially exists.
    /// @return bool Whether that bookmark exists.
    pub fn has_bookmark(&self, bookmark_name: &str) -> bool {
        todo!()
        // self.tag_processor.has_bookmark( "_{$bookmark_name}" )
    }

    ///
    ///
    /// HTML Parsing Algorithms
    ///
    ///

    /// Closes a P element.
    ///
    /// @since 6.4.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#close-a-p-element
    fn close_a_p_element(&mut self) -> () {
        todo!()
    }

    /// Closes elements that have implied end tags.
    ///
    /// @since 6.4.0
    /// @since 6.7.0 Full spec support.
    ///
    /// @see https://html.spec.whatwg.org/#generate-implied-end-tags
    ///
    /// @param string|null $except_for_this_element Perform as if this element doesn't exist in the stack of open elements.
    fn generate_implied_end_tags(&mut self, except_for_this_element: Option<TagName>) -> () {
        todo!()
    }

    /// Closes elements that have implied end tags, thoroughly.
    ///
    /// See the HTML specification for an explanation why this is
    /// different from generating end tags in the normal sense.
    ///
    /// @since 6.4.0
    /// @since 6.7.0 Full spec support.
    ///
    /// @see WP_HTML_Processor::generate_implied_end_tags
    /// @see https://html.spec.whatwg.org/#generate-implied-end-tags
    fn generate_implied_end_tags_thoroughly(&mut self) -> () {
        todo!()
    }

    /// Returns the adjusted current node.
    ///
    /// > The adjusted current node is the context element if the parser was created as
    /// > part of the HTML fragment parsing algorithm and the stack of open elements
    /// > has only one element in it (fragment case); otherwise, the adjusted current
    /// > node is the current node.
    ///
    /// @see https://html.spec.whatwg.org/#adjusted-current-node
    ///
    /// @since 6.7.0
    ///
    /// @return WP_HTML_Token|null The adjusted current node.
    fn get_adjusted_current_node(&self) -> Option<HTMLToken> {
        todo!()
    }

    /// Reconstructs the active formatting elements.
    ///
    /// > This has the effect of reopening all the formatting elements that were opened
    /// > in the current body, cell, or caption (whichever is youngest) that haven't
    /// > been explicitly closed.
    ///
    /// @since 6.4.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#reconstruct-the-active-formatting-elements
    ///
    /// @return bool Whether any formatting elements needed to be reconstructed.

    fn reconstruct_active_formatting_elements(&mut self) -> bool {
        todo!()
    }

    /// Runs the reset the insertion mode appropriately algorithm.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#reset-the-insertion-mode-appropriately

    fn reset_insertion_mode_appropriately(&mut self) -> () {
        todo!()
    }

    /// Runs the adoption agency algorithm.
    ///
    /// @since 6.4.0
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#adoption-agency-algorithm

    fn run_adoption_agency_algorithm(&mut self) -> () {
        todo!()
    }

    /// Runs the "close the cell" algorithm.
    ///
    /// > Where the steps above say to close the cell, they mean to run the following algorithm:
    /// >   1. Generate implied end tags.
    /// >   2. If the current node is not now a td element or a th element, then this is a parse error.
    /// >   3. Pop elements from the stack of open elements stack until a td element or a th element has been popped from the stack.
    /// >   4. Clear the list of active formatting elements up to the last marker.
    /// >   5. Switch the insertion mode to "in row".
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#close-the-cell
    ///
    /// @since 6.7.0

    fn close_cell(&mut self) -> () {
        todo!()
    }

    /// Inserts an HTML element on the stack of open elements.
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#insert-a-foreign-element
    ///
    /// @param WP_HTML_Token $token Name of bookmark pointing to element in original input HTML.

    fn insert_html_element(&mut self, token: HTMLToken) -> () {
        self.state.stack_of_open_elements.push(token);
    }

    /// Inserts a foreign element on to the stack of open elements.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#insert-a-foreign-element
    ///
    /// @param WP_HTML_Token $token                     Insert this token. The token's namespace and
    ///                                                 insertion point will be updated correctly.
    /// @param bool          $only_add_to_element_stack Whether to skip the "insert an element at the adjusted
    ///                                                 insertion location" algorithm when adding this element.
    fn insert_foreign_element(&mut self, token: HTMLToken, only_add_to_element_stack: bool) -> () {
        todo!()
    }

    /// Inserts a virtual element on the stack of open elements.
    ///
    /// @since 6.7.0
    ///
    /// @param string      $token_name    Name of token to create and insert into the stack of open elements.
    /// @param string|null $bookmark_name Optional. Name to give bookmark for created virtual node.
    ///                                   Defaults to auto-creating a bookmark name.
    /// @return WP_HTML_Token Newly-created virtual token.
    fn insert_virtual_node(&mut self, token_name: &str, bookmark_name: Option<&str>) -> HTMLToken {
        todo!()
    }

    ///
    ///
    /// HTML Specification Helpers
    ///
    ///

    /// Indicates if the current token is a MathML integration point.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#mathml-text-integration-point
    ///
    /// @return bool Whether the current token is a MathML integration point.

    fn is_mathml_integration_point(&self) -> bool {
        todo!()
    }

    /// Indicates if the current token is an HTML integration point.
    ///
    /// Note that this method must be an instance method with access
    /// to the current token, since it needs to examine the attributes
    /// of the currently-matched tag, if it's in the MathML namespace.
    /// Otherwise it would be required to scan the HTML and ensure that
    /// no other accounting is overlooked.
    ///
    /// @since 6.7.0
    ///
    /// @see https://html.spec.whatwg.org/#html-integration-point
    ///
    /// @return bool Whether the current token is an HTML integration point.

    fn is_html_integration_point(&self) -> bool {
        todo!()
    }

    /// Returns whether an element of a given name is in the HTML special category.
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#special
    ///
    /// @param WP_HTML_Token|string $tag_name Node to check, or only its name if in the HTML namespace.
    /// @return bool Whether the element of the given name is in the special category.

    pub fn is_special(tag_name: TagName) -> bool {
        todo!()
    }

    /// Returns whether a given element is an HTML Void Element
    ///
    /// > area, base, br, col, embed, hr, img, input, link, meta, source, track, wbr
    ///
    /// @since 6.4.0
    ///
    /// @see https://html.spec.whatwg.org/#void-elements
    ///
    /// @param string $tag_name Name of HTML tag to check.
    /// @return bool Whether the given tag is an HTML Void Element.

    pub fn is_void(tag_name: TagName) -> bool {
        todo!()
    }

    /// Gets an encoding from a given string.
    ///
    /// This is an algorithm defined in the WHAT-WG specification.
    ///
    /// Example:
    ///
    ///     'UTF-8' === self::get_encoding( 'utf8' );
    ///     'UTF-8' === self::get_encoding( "  \tUTF-8 " );
    ///     null    === self::get_encoding( 'UTF-7' );
    ///     null    === self::get_encoding( 'utf8; charset=' );
    ///
    /// @see https://encoding.spec.whatwg.org/#concept-encoding-get
    ///
    /// @todo As this parser only supports UTF-8, only the UTF-8
    ///       encodings are detected. Add more as desired, but the
    ///       parser will bail on non-UTF-8 encodings.
    ///
    /// @since 6.7.0
    ///
    /// @param string $label A string which may specify a known encoding.
    /// @return string|null Known encoding if matched, otherwise null.
    ///
    /// @todo What do wo with this _protected_ function?
    fn get_encoding(label: &str) -> Option<Box<str>> {
        todo!()
    }
}
enum NodeToProcess {
    ProcessNextNode,
    ReprocessCurrentNode,
}
