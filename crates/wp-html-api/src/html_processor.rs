#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod active_formatting_elements;
mod html_stack_event;
mod html_token;
mod stack_of_open_elements;

use std::{collections::VecDeque, rc::Rc};

use crate::tag_processor::{
    AttributeValue, CommentType, CompatMode, HtmlSpan, NodeName, ParserState, ParsingNamespace,
    TagName, TagProcessor, TextNodeClassification, TokenType,
};
use active_formatting_elements::*;
use html_stack_event::*;
use html_token::*;
use stack_of_open_elements::*;

#[derive(PartialEq)]
enum NodeToProcess {
    ProcessNextNode,
    ReprocessCurrentNode,
}

#[derive(Default)]
pub struct TagQuery<'a> {
    tag_name: Option<TagName>,
    tag_closers: VisitClosers,
    match_offset: Option<usize>,
    class_name: Option<&'a str>,
    breadcrumbs: Option<Vec<&'a str>>,
}
#[derive(Default, PartialEq)]
pub enum VisitClosers {
    Visit,
    #[default]
    Skip,
}
pub struct ProcessorState {
    active_formatting_elements: ActiveFormattingElements,
    current_token: Option<HTMLToken>,
    encoding: Rc<str>,
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

#[derive(PartialEq)]
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

pub struct HtmlProcessor {
    tag_processor: TagProcessor,
    state: ProcessorState,
    last_error: Option<HtmlProcessorError>,
    unsupported_exception: Option<String>,
    element_queue: VecDeque<HTMLStackEvent>,
    current_element: Option<HTMLStackEvent>,
    breadcrumbs: Vec<NodeName>,
    bookmark_counter: u32,

    /// Context node if created as a fragment parser.
    context_node: Option<HTMLToken>,
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
    pub fn create_fragment(
        html: &str,
        context: &str,
        known_definite_encoding: &str,
    ) -> Option<Self> {
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
    pub fn create_full_parser(html: &[u8], known_definite_encoding: &str) -> Option<Self> {
        if "UTF-8" != known_definite_encoding {
            return None;
        }

        let mut processor = Self::new(html);
        processor.state.encoding = "UTF-8".into();
        processor.state.encoding_confidence = EncodingConfidence::Certain;

        Some(processor)
    }

    fn new(html: &[u8]) -> Self {
        let tag_processor = TagProcessor::new(html);
        let state = ProcessorState::new();

        // TODO stack push/pop handlers???

        Self {
            tag_processor,
            state,
            element_queue: VecDeque::new(),
            last_error: None,
            unsupported_exception: None,
            current_element: None,
            breadcrumbs: Vec::new(),
            bookmark_counter: 0,
            context_node: None,
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
    pub fn get_last_error(&self) -> Option<&HtmlProcessorError> {
        self.last_error.as_ref()
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

    pub fn get_unsupported_exception(&self) -> Option<&UnsupportedException> {
        match &self.last_error {
            Some(HtmlProcessorError::UnsupportedException(e)) => Some(e),
            _ => None,
        }
    }

    /// Finds the next tag matching the query.
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
    pub fn next_tag(&mut self, query: Option<TagQuery>) -> bool {
        // Handle null/None query case
        if query.is_none() {
            while self.next_token() {
                if self.get_token_type() != Some(TokenType::Tag) {
                    continue;
                }

                if !self.is_tag_closer() {
                    return true;
                }
            }
            return false;
        }

        let query = query.unwrap();
        let visit_closers = query.tag_closers == VisitClosers::Visit;

        if query.breadcrumbs.is_none() {
            while self.next_token() {
                if self.get_token_type() != Some(TokenType::Tag) {
                    continue;
                }

                if let Some(tag_name) = &query.tag_name {
                    if self.get_tag().unwrap() != *tag_name {
                        continue;
                    }
                }

                if let Some(class_name) = query.class_name {
                    if !self.has_class(class_name).unwrap_or(false) {
                        continue;
                    }
                }

                if !self.is_tag_closer() || visit_closers {
                    return true;
                }
            }

            return false;
        }

        let breadcrumbs = &query.breadcrumbs;
        let mut match_offset = query.match_offset.unwrap_or(0);

        while match_offset > 0 && self.next_token() {
            if self.get_token_type() != Some(TokenType::Tag) || self.is_tag_closer() {
                continue;
            }

            if let Some(class_name) = query.class_name {
                if !self.has_class(class_name).unwrap_or(false) {
                    continue;
                }
            }

            if self.matches_breadcrumbs(breadcrumbs.as_ref()) {
                if match_offset < 1 {
                    return true;
                } else {
                    match_offset -= 1;
                }
            }
        }

        false
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
    /// @return bool
    fn next_visitable_token(&mut self) -> bool {
        self.current_element = None;

        if self.last_error.is_some() {
            return false;
        }

        /*
         * Prime the events if there are none.
         *
         * @todo In some cases, probably related to the adoption agency
         *       algorithm, this call to step() doesn't create any new
         *       events. Calling it again creates them. Figure out why
         *       this is and if it's inherent or if it's a bug. Looping
         *       until there are events or until there are no more
         *       tokens works in the meantime and isn't obviously wrong.
         */
        if self.element_queue.is_empty() && self.step(NodeToProcess::ProcessNextNode) {
            return self.next_visitable_token();
        }

        // Process the next event on the queue
        self.current_element = self.element_queue.pop_front();
        if self.current_element.is_none() {
            // There are no tokens left, so close all remaining open elements
            while self.state.stack_of_open_elements.pop().is_some() {
                continue;
            }

            return if self.element_queue.is_empty() {
                false
            } else {
                self.next_visitable_token()
            };
        }

        let current_element = self.current_element.as_ref().unwrap();
        let is_pop = current_element.operation == StackOperation::Pop;

        // The root node only exists in the fragment parser, and closing it
        // indicates that the parse is complete. Stop before popping it from
        // the breadcrumbs.
        if current_element
            .token
            .bookmark_name
            .as_ref()
            .map_or(false, |name| name.as_ref() == "root-node")
        {
            return self.next_visitable_token();
        }

        // Adjust the breadcrumbs for this event
        if is_pop {
            self.breadcrumbs.pop();
        } else {
            self.breadcrumbs
                .push(current_element.token.node_name.clone());
        }

        // Avoid sending close events for elements which don't expect a closing
        if is_pop && !self.expects_closer(Some(&current_element.token)) {
            return self.next_visitable_token();
        }

        true
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

    pub fn matches_breadcrumbs(&self, breadcrumbs: Option<&Vec<&str>>) -> bool {
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

    pub fn expects_closer(&self, node: Option<&HTMLToken>) -> bool {
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
    fn step(&mut self, node_to_process: NodeToProcess) -> bool {
        // Refuse to proceed if there was a previous error.
        if self.last_error.is_some() {
            return false;
        }

        if node_to_process != NodeToProcess::ReprocessCurrentNode {
            /*
             * Void elements still hop onto the stack of open elements even though
             * there's no corresponding closing tag. This is important for managing
             * stack-based operations such as "navigate to parent node" or checking
             * on an element's breadcrumbs.
             *
             * When moving on to the next node, therefore, if the bottom-most element
             * on the stack is a void element, it must be closed.
             */
            if let Some(top_node) = self.state.stack_of_open_elements.current_node() {
                if !self.expects_closer(Some(top_node)) {
                    self.state.stack_of_open_elements.pop();
                }
            }
        }

        if node_to_process == NodeToProcess::ProcessNextNode {
            self.tag_processor.next_token();
            if self.tag_processor.parser_state == ParserState::TextNode {
                self.tag_processor.subdivide_text_appropriately();
            }
        }

        // Finish stepping when there are no more tokens in the document.
        if matches!(
            self.tag_processor.parser_state,
            ParserState::IncompleteInput | ParserState::Complete
        ) {
            return false;
        }

        let token_name = self.get_token_name().unwrap();
        if node_to_process != NodeToProcess::ReprocessCurrentNode {
            if let Ok(bookmark) = self.bookmark_token() {
                self.state.current_token = Some(HTMLToken::new(
                    Some(bookmark.as_ref()),
                    token_name.clone(),
                    self.has_self_closing_flag(),
                ));
            } else {
                return false;
            }
        }

        let parse_in_current_insertion_mode = self.state.stack_of_open_elements.count() == 0
            || {
                let adjusted_current_node = self.get_adjusted_current_node().unwrap();
                let is_closer = self.is_tag_closer();
                let is_start_tag =
                    self.tag_processor.parser_state == ParserState::MatchedTag && !is_closer;

                adjusted_current_node.namespace == ParsingNamespace::Html
                    || (adjusted_current_node.integration_node_type
                        == Some(IntegrationNodeType::MathML)
                        && ((is_start_tag
                            && (!matches!( &token_name, NodeName::Tag( TagName::Arbitrary(arbitrary_name) ) if &**arbitrary_name == b"MGLYPH" || &**arbitrary_name == b"MALIGNMARK"  )))
                            || token_name == TokenType::Text.into()))
                    || (adjusted_current_node.namespace == ParsingNamespace::MathML
                        && matches!(
                            &adjusted_current_node.node_name, NodeName::Tag(TagName::Arbitrary(arbitrary_name)) if &**arbitrary_name == b"ANNOTATION-XML")
                        && is_start_tag
                        && matches!(
                            &adjusted_current_node.node_name, NodeName::Tag(TagName::Arbitrary(arbitrary_name)) if &**arbitrary_name == b"SVG"))
                    || (adjusted_current_node.integration_node_type
                        == Some(IntegrationNodeType::HTML)
                        && (is_start_tag || token_name == TokenType::Text.into()))
            };

        let step_result = if !parse_in_current_insertion_mode {
            self.step_in_foreign_content()
        } else {
            match self.state.insertion_mode {
                InsertionMode::INITIAL => self.step_initial(),
                InsertionMode::BEFORE_HTML => self.step_before_html(),
                InsertionMode::BEFORE_HEAD => self.step_before_head(),
                InsertionMode::IN_HEAD => self.step_in_head(),
                InsertionMode::IN_HEAD_NOSCRIPT => self.step_in_head_noscript(),
                InsertionMode::AFTER_HEAD => self.step_after_head(),
                InsertionMode::IN_BODY => self.step_in_body(),
                InsertionMode::IN_TABLE => self.step_in_table(),
                InsertionMode::IN_TABLE_TEXT => self.step_in_table_text(),
                InsertionMode::IN_CAPTION => self.step_in_caption(),
                InsertionMode::IN_COLUMN_GROUP => self.step_in_column_group(),
                InsertionMode::IN_TABLE_BODY => self.step_in_table_body(),
                InsertionMode::IN_ROW => self.step_in_row(),
                InsertionMode::IN_CELL => self.step_in_cell(),
                InsertionMode::IN_SELECT => self.step_in_select(),
                InsertionMode::IN_SELECT_IN_TABLE => self.step_in_select_in_table(),
                InsertionMode::IN_TEMPLATE => self.step_in_template(),
                InsertionMode::AFTER_BODY => self.step_after_body(),
                InsertionMode::IN_FRAMESET => self.step_in_frameset(),
                InsertionMode::AFTER_FRAMESET => self.step_after_frameset(),
                InsertionMode::AFTER_AFTER_BODY => self.step_after_after_body(),
                InsertionMode::AFTER_AFTER_FRAMESET => self.step_after_after_frameset(),
            }
        };

        // @todo use Results
        step_result
        //match step_result {
        //    Ok(result) => result,
        //    Err(_) => false,
        //}
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
        let processor = Self::create_fragment(html, "<body>", "UTF-8")
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
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * Parse error: ignore the token.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                return self.step(NodeToProcess::ProcessNextNode).into();
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                let token: HTMLToken = self.state.current_token.clone().unwrap();
                self.insert_html_element(token);
                return true;
            }

            /*
             * > A DOCTYPE token
             */
            Op::Token(TokenType::Doctype) => {
                todo!("Doctype token handling");

                // $doctype = $this->get_doctype_info();
                // if ( null !== $doctype && 'quirks' === $doctype->indicated_compatability_mode ) {
                // 	$this->compat_mode = WP_HTML_Tag_Processor::QUIRKS_MODE;
                // }

                // /*
                //  * > Then, switch the insertion mode to "before html".
                //  */
                // $this->state->insertion_mode = WP_HTML_Processor_State::INSERTION_MODE_BEFORE_HTML;
                // $this->insert_html_element( $this->state->current_token );
                // return true;
            }
            /*
             * > Anything else
             */
            _ => {
                self.tag_processor.compat_mode = CompatMode::Quirks;
                self.state.insertion_mode = InsertionMode::BEFORE_HTML;
                self.step(NodeToProcess::ReprocessCurrentNode).into()
            }
        }
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
        match self.make_op() {
            /*
             * > A DOCTYPE token
             */
            Op::Token(TokenType::Doctype) => return self.step(NodeToProcess::ProcessNextNode),

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * Parse error: ignore the token.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::BEFORE_HEAD;
                true
            }

            /*
             * > An end tag whose tag name is one of: "head", "body", "html", "br"
             *   > Act as described in the "anything else" entry below.
             * > Any other end tag
             *
             * Closing BR tags are always reported by the Tag Processor as opening tags.
             */
            Op::TagPop(tag_name)
                if matches!(
                    tag_name,
                    TagName::HEAD | TagName::BODY | TagName::HTML | TagName::BR,
                ) =>
            {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else.
             *
             * > Create an html element whose node document is the Document object.
             * > Append it to the Document object. Put this element in the stack of open elements.
             * > Switch the insertion mode to "before head", then reprocess the token.
             */
            _ => {
                self.insert_virtual_node(TagName::HTML, None);
                self.state.insertion_mode = InsertionMode::BEFORE_HEAD;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
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
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * Parse error: ignore the token.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                let token: HTMLToken = self.state.current_token.clone().unwrap();
                self.insert_html_element(token);
                true
            }

            /*
             * > A DOCTYPE token
             */
            Op::Token(TokenType::Doctype) => self.step(NodeToProcess::ProcessNextNode),

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > A start tag whose tag name is "head"
             */
            Op::TagPush(TagName::HEAD) => {
                let token: HTMLToken = self.state.current_token.clone().unwrap();
                self.insert_html_element(token.clone());
                self.state.head_element = Some(token);
                self.state.insertion_mode = InsertionMode::IN_HEAD;
                true
            }

            /*
             * > An end tag whose tag name is one of: "head", "body", "html", "br"
             *   > Act as described in the "anything else" entry below.
             * > Any other end tag
             *
             * Closing BR tags are always reported by the Tag Processor as opening tags.
             */
            Op::TagPop(tag_name)
                if matches!(
                    tag_name,
                    TagName::HEAD | TagName::BODY | TagName::HTML | TagName::BR,
                ) =>
            {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             *
             * > Insert an HTML element for a "head" start tag token with no attributes.
             */
            _ => {
                self.insert_virtual_node(TagName::HEAD, None);
                self.state.insertion_mode = InsertionMode::IN_HEAD;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
    }

    /// Parses next element in the 'in head' insertion mode.
    ///
    /// This internal function performs the 'in head' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_head(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * Parse error: ignore the token.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                // Insert the character.
                let token: HTMLToken = self.state.current_token.clone().unwrap();
                self.insert_html_element(token.clone());
                true
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A DOCTYPE token
             *
            	* Parse error: ignore the token.
             */
            Op::Token(TokenType::Doctype) => self.step(NodeToProcess::ProcessNextNode),

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > A start tag whose tag name is one of: "base", "basefont", "bgsound", "link"
             */
            Op::TagPush(TagName::BASE | TagName::BASEFONT | TagName::BGSOUND | TagName::LINK) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "meta"
             */
            Op::TagPush(TagName::META) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());

                /*
                 * > If the active speculative HTML parser is null, then:
                 * >   - If the element has a charset attribute, and getting an encoding from
                 * >     its value results in an encoding, and the confidence is currently
                 * >     tentative, then change the encoding to the resulting encoding.
                 */
                if let AttributeValue::String(_) = self.get_attribute("charset") {
                    if EncodingConfidence::Tentative == self.state.encoding_confidence {
                        self.bail(
                            "Cannot yet process META tags with charset to determine encoding."
                                .to_string(),
                        );
                    }
                    todo!()
                }

                /*
                 * >   - Otherwise, if the element has an http-equiv attribute whose value is
                 * >     an ASCII case-insensitive match for the string "Content-Type", and
                 * >     the element has a content attribute, and applying the algorithm for
                 * >     extracting a character encoding from a meta element to that attribute's
                 * >     value returns an encoding, and the confidence is currently tentative,
                 * >     then change the encoding to the extracted encoding.
                 */

                if let (AttributeValue::String(http_equiv), AttributeValue::String(_)) = (
                    self.get_attribute("http-equiv"),
                    self.get_attribute("content"),
                ) {
                    if http_equiv.eq_ignore_ascii_case(b"Content-Type")
                        && self.state.encoding_confidence == EncodingConfidence::Tentative
                    {
                        self.bail( "Cannot yet process META tags with http-equiv Content-Type to determine encoding.".to_string() );
                    }
                }

                true
            }

            /*
             * > A start tag whose tag name is "title"
             */
            Op::TagPush(TagName::TITLE) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "noscript", if the scripting flag is enabled
             * > A start tag whose tag name is one of: "noframes", "style"
             *
             * The scripting flag is never enabled in this parser.
             */
            Op::TagPush(TagName::NOFRAMES | TagName::STYLE) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "noscript", if the scripting flag is disabled
             */
            Op::TagPush(TagName::NOSCRIPT) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_HEAD_NOSCRIPT;
                true
            }

            /*
             * > A start tag whose tag name is "script"
             *
             * @todo Could the adjusted insertion location be anything other than the current location?
             */
            Op::TagPush(TagName::SCRIPT) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > An end tag whose tag name is "head"
             */
            Op::TagPop(TagName::HEAD) => {
                self.state.stack_of_open_elements.pop();
                self.state.insertion_mode = InsertionMode::AFTER_HEAD;
                true
            }

            /*
             * > An end tag whose tag name is one of: "body", "html", "br"
             *
             * This rule will be implemented a guard on the "any other close tag" rule below.
             *
             * BR tags are always reported by the Tag Processor as opening tags.
             */

            /*
             * > A start tag whose tag name is "template"
             *
             * @todo Could the adjusted insertion location be anything other than the current location?
             */
            Op::TagPush(TagName::TEMPLATE) => {
                self.state.active_formatting_elements.insert_marker();
                self.state.frameset_ok = false;

                self.state.insertion_mode = InsertionMode::IN_TEMPLATE;
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_TEMPLATE);

                self.insert_html_element(self.state.current_token.clone().unwrap());
                return true;
            }

            /*
             * > An end tag whose tag name is "template"
             */
            Op::TagPop(TagName::TEMPLATE) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .contains(&TagName::TEMPLATE)
                {
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.generate_implied_end_tags_thoroughly();
                    // @todo If the current node is not a TEMPLATE elemnt, then
                    // indicate a parse error once it's possible.
                    self.state
                        .stack_of_open_elements
                        .pop_until(&TagName::TEMPLATE);
                    self.state
                        .active_formatting_elements
                        .clear_up_to_last_marker();
                    self.state.stack_of_template_insertion_modes.pop();
                    true
                }
            }
            /*
             * > A start tag whose tag name is "head"
             * > Any other end tag
             *
             * This includes handling for the end tag rules for BODY and HTML elements above that should
             * fall through to the anything else case below.
             *
             * Parse error: ignore the token.
             */
            Op::TagPush(TagName::HEAD) => self.step(NodeToProcess::ProcessNextNode),
            Op::TagPop(tag_name) if !matches!(tag_name, TagName::BODY | TagName::HTML) => {
                self.step(NodeToProcess::ProcessNextNode)
            }

            Op::TagPop(TagName::BODY | TagName::HTML) => {
                /*
                 * > Act as described in the "anything else" entry below.
                 */
                todo!();
            }

            /*
             * > Anything else
             */
            _ => {
                self.state.stack_of_open_elements.pop();
                self.state.insertion_mode = InsertionMode::AFTER_HEAD;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
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
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * Parse error: ignore the token.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_head()
            }

            /*
             * > A DOCTYPE token
             */
            Op::Token(TokenType::Doctype) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > An end tag whose tag name is "noscript"
             */
            Op::TagPop(TagName::NOSCRIPT) => {
                self.state.stack_of_open_elements.pop();
                self.state.insertion_mode = InsertionMode::IN_HEAD;
                true
            }

            /*
             * > A comment token
             * >
             * > A start tag whose tag name is one of: "basefont", "bgsound",
             * > "link", "meta", "noframes", "style"
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            )
            | Op::TagPush(
                TagName::BASEFONT
                | TagName::BGSOUND
                | TagName::LINK
                | TagName::META
                | TagName::NOFRAMES
                | TagName::STYLE,
            ) => self.step_in_head(),

            /*
             * > An end tag whose tag name is "br"
             *
             * This should never happen, as the Tag Processor prevents showing a BR closing tag.
             */

            /*
             * > A start tag whose tag name is one of: "head", "noscript"
             * > Any other end tag
             */
            Op::TagPush(TagName::HEAD | TagName::NOSCRIPT) | Op::TagPop(_) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             *
             * Anything here is a parse error.
             */
            _ => {
                self.state.stack_of_open_elements.pop();
                self.state.insertion_mode = InsertionMode::IN_HEAD;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
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
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                // Insert the character.
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A DOCTYPE token
             */
            Op::Token(TokenType::Doctype) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > A start tag whose tag name is "body"
             */
            Op::TagPush(TagName::BODY) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;
                self.state.insertion_mode = InsertionMode::IN_BODY;
                true
            }

            /*
             * > A start tag whose tag name is "frameset"
             */
            Op::TagPush(TagName::FRAMESET) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_FRAMESET;
                true
            }

            /*
             * > A start tag whose tag name is one of: "base", "basefont", "bgsound",
             * > "link", "meta", "noframes", "script", "style", "template", "title"
             *
             * Anything here is a parse error.
             */
            Op::TagPush(
                TagName::BASE
                | TagName::BASEFONT
                | TagName::BGSOUND
                | TagName::LINK
                | TagName::META
                | TagName::NOFRAMES
                | TagName::SCRIPT
                | TagName::STYLE
                | TagName::TEMPLATE
                | TagName::TITLE,
            ) => {
                /*
                 * > Push the node pointed to by the head element pointer onto the stack of open elements.
                 * > Process the token using the rules for the "in head" insertion mode.
                 * > Remove the node pointed to by the head element pointer from the stack of open elements. (It might not be the current node at this point.)
                 */
                self.bail(
                    "Cannot process elements after HEAD which reopen the HEAD element.".to_string(),
                );
                todo!()
            }

            /*
             * > An end tag whose tag name is "template"
             */
            Op::TagPop(TagName::TEMPLATE) => self.step_in_head(),

            /*
             * > An end tag whose tag name is one of: "body", "html", "br"
             *
             * This rule will be implemented a guard on the "any other close tag" rule below.
             *
             * BR tags are always reported by the Tag Processor as opening tags.
             */

            /*
             * > A start tag whose tag name is "head"
             * > Any other end tag
             *
                          * This includes handling for the end tag rules for BODY and HTML elements above that should
             * fall through to the anything else case below.
             *
             * Parse error: ignore the token.
             */
            Op::TagPush(TagName::HEAD) => self.step(NodeToProcess::ProcessNextNode),
            Op::TagPop(tag_name) if !matches!(tag_name, TagName::BODY | TagName::HTML) => {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             */
            _ => {
                self.insert_virtual_node(TagName::BODY, None);
                self.state.insertion_mode = InsertionMode::IN_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
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
        match self.make_op() {
            /*
             * > A character token that is U+0000 NULL
             *
             * Any successive sequence of NULL bytes is ignored and won't
             * trigger active format reconstruction. Therefore, if the text
             * only comprises NULL bytes then the token should be ignored
             * here, but if there are any other characters in the stream
             * the active formats should be reconstructed.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::NullSequence =>
            {
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION,
             * > U+000A LINE FEED (LF), U+000C FORM FEED (FF),
             * > U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * > Any other character token
             */
            Op::Token(TokenType::Text) => {
                self.reconstruct_active_formatting_elements();

                /*
                 * Whitespace-only text does not affect the frameset-ok flag.
                 * It is probably inter-element whitespace, but it may also
                 * contain character references which decode only to whitespace.
                 */
                if self.tag_processor.text_node_classification == TextNodeClassification::Generic {
                    self.state.frameset_ok = false;
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A DOCTYPE token
             * > Parse error. Ignore the token.
             */
            Op::Token(TokenType::Doctype) => self.step(NodeToProcess::ProcessNextNode),

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .contains(&TagName::TEMPLATE)
                {
                    /*
                     * > Otherwise, for each attribute on the token, check to see if the attribute
                     * > is already present on the top element of the stack of open elements. If
                     * > it is not, add the attribute and its corresponding value to that element.
                     *
                     * This parser does not currently support this behavior: ignore the token.
                     */
                }

                // Ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is one of: "base", "basefont", "bgsound", "link",
             * > "meta", "noframes", "script", "style", "template", "title"
             * >
             * > An end tag whose tag name is "template"
             */
            Op::TagPush(
                TagName::BASE
                | TagName::BASEFONT
                | TagName::BGSOUND
                | TagName::LINK
                | TagName::META
                | TagName::NOFRAMES
                | TagName::SCRIPT
                | TagName::STYLE
                | TagName::TEMPLATE
                | TagName::TITLE,
            )
            | Op::TagPop(TagName::TEMPLATE) => self.step_in_head(),

            /*
             * > A start tag whose tag name is "body"
             *
             * This tag in the IN BODY insertion mode is a parse error.
             */
            Op::TagPush(TagName::BODY) => {
                if 1 == self.state.stack_of_open_elements.count()
                    || !matches!(
                        self.state.stack_of_open_elements.at(2),
                        Some(HTMLToken {
                            node_name: NodeName::Tag(TagName::BODY),
                            ..
                        })
                    )
                    || self
                        .state
                        .stack_of_open_elements
                        .contains(&TagName::TEMPLATE)
                {
                    // Ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    /*
                     * > Otherwise, set the frameset-ok flag to "not ok"; then, for each attribute
                     * > on the token, check to see if the attribute is already present on the body
                     * > element (the second element) on the stack of open elements, and if it is
                     * > not, add the attribute and its corresponding value to that element.
                     *
                     * This parser does not currently support this behavior: ignore the token.
                     */
                    self.state.frameset_ok = false;
                    self.step(NodeToProcess::ProcessNextNode)
                }
            }

            /*
             * > A start tag whose tag name is "frameset"
             *
             * This tag in the IN BODY insertion mode is a parse error.
             */
            Op::TagPush(TagName::FRAMESET) => {
                if 1 == self.state.stack_of_open_elements.count()
                    || !matches!(
                        self.state.stack_of_open_elements.at(2),
                        Some(HTMLToken {
                            node_name: NodeName::Tag(TagName::BODY),
                            ..
                        })
                    )
                    || !self.state.frameset_ok
                {
                    // Ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    /*
                     * > Otherwise, run the following steps:
                     */
                    self.bail("Cannot process non-ignored FRAMESET tags.".to_string());
                    todo!()
                }
            }

            /*
             * > An end tag whose tag name is "body"
             */
            Op::TagPop(TagName::BODY) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::BODY)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    /*
                     * > Otherwise, if there is a node in the stack of open elements that is not either a
                     * > dd element, a dt element, an li element, an optgroup element, an option element,
                     * > a p element, an rb element, an rp element, an rt element, an rtc element, a tbody
                     * > element, a td element, a tfoot element, a th element, a thread element, a tr
                     * > element, the body element, or the html element, then this is a parse error.
                     *
                     * There is nothing to do for this parse error, so don't check for it.
                     */

                    self.state.insertion_mode = InsertionMode::AFTER_BODY;
                    /*
                     * The BODY element is not removed from the stack of open elements.
                     * Only internal state has changed, this does not qualify as a "step"
                     * in terms of advancing through the document to another token.
                     * Nothing has been pushed or popped.
                     * Proceed to parse the next item.
                     */
                    self.step(NodeToProcess::ProcessNextNode)
                }
            }

            /*
             * > An end tag whose tag name is "html"
             */
            Op::TagPop(TagName::HTML) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::BODY)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    /*
                     * > Otherwise, if there is a node in the stack of open elements that is not either a
                     * > dd element, a dt element, an li element, an optgroup element, an option element,
                     * > a p element, an rb element, an rp element, an rt element, an rtc element, a tbody
                     * > element, a td element, a tfoot element, a th element, a thread element, a tr
                     * > element, the body element, or the html element, then this is a parse error.
                     *
                     * There is nothing to do for this parse error, so don't check for it.
                     */

                    self.state.insertion_mode = InsertionMode::AFTER_BODY;
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }

            /*
             * > A start tag whose tag name is one of: "address", "article", "aside",
             * > "blockquote", "center", "details", "dialog", "dir", "div", "dl",
             * > "fieldset", "figcaption", "figure", "footer", "header", "hgroup",
             * > "main", "menu", "nav", "ol", "p", "search", "section", "summary", "ul"
             */
            Op::TagPush(
                TagName::ADDRESS
                | TagName::ARTICLE
                | TagName::ASIDE
                | TagName::BLOCKQUOTE
                | TagName::CENTER
                | TagName::DETAILS
                | TagName::DIALOG
                | TagName::DIR
                | TagName::DIV
                | TagName::DL
                | TagName::FIELDSET
                | TagName::FIGCAPTION
                | TagName::FIGURE
                | TagName::FOOTER
                | TagName::HEADER
                | TagName::HGROUP
                | TagName::MAIN
                | TagName::MENU
                | TagName::NAV
                | TagName::OL
                | TagName::P
                | TagName::SEARCH
                | TagName::SECTION
                | TagName::SUMMARY
                | TagName::UL,
            ) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6"
             */
            Op::TagPush(
                TagName::H1 | TagName::H2 | TagName::H3 | TagName::H4 | TagName::H5 | TagName::H6,
            ) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                if let Some(HTMLToken {
                    node_name: NodeName::Tag(tag_name),
                    ..
                }) = self.state.stack_of_open_elements.current_node()
                {
                    if matches!(
                        tag_name,
                        TagName::H1
                            | TagName::H2
                            | TagName::H3
                            | TagName::H4
                            | TagName::H5
                            | TagName::H6
                    ) {
                        // Parse error: pop the current heading element
                        self.state.stack_of_open_elements.pop();
                    }
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is one of: "pre", "listing"
             */
            Op::TagPush(TagName::PRE | TagName::LISTING) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                /*
                 * > If the next token is a U+000A LINE FEED (LF) character token,
                 * > then ignore that token and move on to the next one. (Newlines
                 * > at the start of pre blocks are ignored as an authoring convenience.)
                 *
                 * This is handled in `get_modifiable_text()`.
                 */

                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;
                true
            }

            /*
             * > A start tag whose tag name is "form"
             */
            Op::TagPush(TagName::FORM) => {
                let stack_contains_template = self
                    .state
                    .stack_of_open_elements
                    .contains(&TagName::TEMPLATE);

                if self.state.form_element.is_some() && !stack_contains_template {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    if self.state.stack_of_open_elements.has_p_in_button_scope() {
                        self.close_a_p_element();
                    }

                    self.insert_html_element(self.state.current_token.clone().unwrap());
                    if !stack_contains_template {
                        self.state.form_element = self.state.current_token.clone();
                    }

                    true
                }
            }

            /*
             * > A start tag whose tag name is "li"
             * > A start tag whose tag name is one of: "dd", "dt"
             */
            Op::TagPush(tag_name @ (TagName::LI | TagName::DD | TagName::DT)) => {
                todo!()
            }

            /*
             * > A start tag whose tag name is "plaintext"
             */
            Op::TagPush(TagName::PLAINTEXT) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                /*
                 * @todo This may need to be handled in the Tag Processor and turn into
                 *       a single self-contained tag like TEXTAREA, whose modifiable text
                 *       is the rest of the input document as plaintext.
                 */
                self.bail("Cannot process PLAINTEXT elements.".to_string());
                todo!()
            }

            /*
             * > A start tag whose tag name is "button"
             */
            Op::TagPush(TagName::BUTTON) => {
                if self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::BUTTON)
                {
                    // Parse error: this error does not impact the logic here.
                    self.generate_implied_end_tags(None);
                    self.state
                        .stack_of_open_elements
                        .pop_until(&TagName::BUTTON);
                }

                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;

                true
            }

            /*
             * > An end tag whose tag name is one of: "address", "article", "aside", "blockquote",
             * > "button", "center", "details", "dialog", "dir", "div", "dl", "fieldset",
             * > "figcaption", "figure", "footer", "header", "hgroup", "listing", "main",
             * > "menu", "nav", "ol", "pre", "search", "section", "summary", "ul"
             */
            Op::TagPop(
                tag_name @ (TagName::ADDRESS
                | TagName::ARTICLE
                | TagName::ASIDE
                | TagName::BLOCKQUOTE
                | TagName::BUTTON
                | TagName::CENTER
                | TagName::DETAILS
                | TagName::DIALOG
                | TagName::DIR
                | TagName::DIV
                | TagName::DL
                | TagName::FIELDSET
                | TagName::FIGCAPTION
                | TagName::FIGURE
                | TagName::FOOTER
                | TagName::HEADER
                | TagName::HGROUP
                | TagName::LISTING
                | TagName::MAIN
                | TagName::MENU
                | TagName::NAV
                | TagName::OL
                | TagName::PRE
                | TagName::SEARCH
                | TagName::SECTION
                | TagName::SUMMARY
                | TagName::UL),
            ) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.generate_implied_end_tags(None);
                    if !self.state.stack_of_open_elements.current_node_is(&tag_name) {
                        // Parse error: this error doesn't impact parsing.
                    }
                    self.state.stack_of_open_elements.pop_until(&tag_name);
                    true
                }
            }

            /*
             * > An end tag whose tag name is "form"
             */
            Op::TagPop(TagName::FORM) => todo!(),

            /*
             * > An end tag whose tag name is "p"
             */
            Op::TagPop(TagName::P) => {
                if !self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.insert_html_element(self.state.current_token.clone().unwrap());
                }

                self.close_a_p_element();
                true
            }

            /*
             * > An end tag whose tag name is "li"
             * > An end tag whose tag name is one of: "dd", "dt"
             */
            Op::TagPop(tag_name @ (TagName::LI | TagName::DD | TagName::DT)) => todo!(),

            /*
             * > An end tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6"
             */
            Op::TagPop(
                tag_name @ (TagName::H1
                | TagName::H2
                | TagName::H3
                | TagName::H4
                | TagName::H5
                | TagName::H6),
            ) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_any_h1_to_h6_element_in_scope()
                {
                    /*
                     * This is a parse error; ignore the token.
                     *
                     * @todo Indicate a parse error once it's possible.
                     */
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.generate_implied_end_tags(None);

                    if !self.state.stack_of_open_elements.current_node_is(&tag_name) {
                        // Parse error: this error doesn't impact parsing.
                    }

                    self.state.stack_of_open_elements.pop_until_any_h1_to_h6();
                    true
                }
            }

            /*
             * > A start tag whose tag name is "a"
             */
            Op::TagPush(TagName::A) => todo!(),

            /*
             * > A start tag whose tag name is one of: "b", "big", "code", "em", "font", "i",
             * > "s", "small", "strike", "strong", "tt", "u"
             */
            Op::TagPush(
                TagName::B
                | TagName::BIG
                | TagName::CODE
                | TagName::EM
                | TagName::FONT
                | TagName::I
                | TagName::S
                | TagName::SMALL
                | TagName::STRIKE
                | TagName::STRONG
                | TagName::TT
                | TagName::U,
            ) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state
                    .active_formatting_elements
                    .push(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "nobr"
             */
            Op::TagPush(TagName::NOBR) => {
                self.reconstruct_active_formatting_elements();

                if self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::NOBR)
                {
                    // Parse error.
                    self.run_adoption_agency_algorithm();
                    self.reconstruct_active_formatting_elements();
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state
                    .active_formatting_elements
                    .push(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > An end tag whose tag name is one of: "a", "b", "big", "code", "em", "font", "i",
             * > "nobr", "s", "small", "strike", "strong", "tt", "u"
             */
            Op::TagPop(
                TagName::A
                | TagName::B
                | TagName::BIG
                | TagName::CODE
                | TagName::EM
                | TagName::FONT
                | TagName::I
                | TagName::NOBR
                | TagName::S
                | TagName::SMALL
                | TagName::STRIKE
                | TagName::STRONG
                | TagName::TT
                | TagName::U,
            ) => {
                self.run_adoption_agency_algorithm();
                true
            }

            /*
             * > A start tag whose tag name is one of: "applet", "marquee", "object"
             */
            Op::TagPush(TagName::APPLET | TagName::MARQUEE | TagName::OBJECT) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.active_formatting_elements.insert_marker();
                self.state.frameset_ok = false;
                true
            }

            /*
             * > A end tag token whose tag name is one of: "applet", "marquee", "object"
             */
            Op::TagPop(tag_name @ (TagName::APPLET | TagName::MARQUEE | TagName::OBJECT)) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.generate_implied_end_tags(None);
                    if !self.state.stack_of_open_elements.current_node_is(&tag_name) {
                        // This is a parse error.
                    }

                    self.state
                        .stack_of_open_elements
                        .pop_until(&self.get_tag().unwrap());
                    self.state
                        .active_formatting_elements
                        .clear_up_to_last_marker();
                    true
                }
            }

            /*
             * > A start tag whose tag name is "table"
             */
            Op::TagPush(TagName::TABLE) => {
                /*
                 * > If the Document is not set to quirks mode, and the stack of open elements
                 * > has a p element in button scope, then close a p element.
                 */
                if self.tag_processor.compat_mode != CompatMode::Quirks
                    && self.state.stack_of_open_elements.has_p_in_button_scope()
                {
                    self.close_a_p_element();
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;
                self.state.insertion_mode = InsertionMode::IN_TABLE;
                true
            }

            /*
             * > An end tag whose tag name is "br"
             *
             * This is prevented from happening because the Tag Processor
             * reports all closing BR tags as if they were opening tags.
             */

            /*
             * > A start tag whose tag name is one of: "area", "br", "embed", "img", "keygen", "wbr"
             */
            Op::TagPush(
                TagName::AREA
                | TagName::BR
                | TagName::EMBED
                | TagName::IMG
                | TagName::KEYGEN
                | TagName::WBR,
            ) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;
                true
            }

            /*
             * > A start tag whose tag name is "input"
             */
            Op::TagPush(TagName::INPUT) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());

                /*
                 * > If the token does not have an attribute with the name "type", or if it does,
                 * > but that attribute's value is not an ASCII case-insensitive match for the
                 * > string "hidden", then: set the frameset-ok flag to "not ok".
                 */
                match self.get_attribute("type") {
                    AttributeValue::String(type_attr_value)
                        if !type_attr_value.eq_ignore_ascii_case(b"hidden") =>
                    {
                        self.state.frameset_ok = false;
                    }
                    AttributeValue::String(_) => {}
                    AttributeValue::BooleanFalse | AttributeValue::BooleanTrue => {
                        self.state.frameset_ok = false;
                    }
                }

                true
            }

            /*
             * > A start tag whose tag name is one of: "param", "source", "track"
             */
            Op::TagPush(TagName::PARAM | TagName::SOURCE | TagName::TRACK) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "hr"
             */
            Op::TagPush(TagName::HR) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;
                true
            }

            /*
               * > A start tag whose tag name is "image"
               * > Parse error. Change the token's tag name to "img" and reprocess it. (Don't ask.)
               *
               * Note that this is handled elsewhere, so it should not be possible to reach this code.
               */


            /*
             * > A start tag whose tag name is "textarea"
             */
            Op::TagPush(TagName::TEXTAREA) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());

                /*
                 * > If the next token is a U+000A LINE FEED (LF) character token, then ignore
                 * > that token and move on to the next one. (Newlines at the start of
                 * > textarea elements are ignored as an authoring convenience.)
                 *
                 * This is handled in `get_modifiable_text()`.
                 */

                self.state.frameset_ok = false;

                /*
                 * > Switch the insertion mode to "text".
                 *
                 * As a self-contained node, this behavior is handled in the Tag Processor.
                 */
                true
            }

            /*
             * > A start tag whose tag name is "xmp"
             */
            Op::TagPush(TagName::XMP) => {
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                self.reconstruct_active_formatting_elements();
                self.state.frameset_ok = false;

                /*
                 * > Follow the generic raw text element parsing algorithm.
                 *
                 * As a self-contained node, this behavior is handled in the Tag Processor.
                 */
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * A start tag whose tag name is "iframe"
             */
            Op::TagPush(TagName::IFRAME) => {
                self.state.frameset_ok = false;

                /*
                 * > Follow the generic raw text element parsing algorithm.
                 *
                 * As a self-contained node, this behavior is handled in the Tag Processor.
                 */
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "noembed"
             * > A start tag whose tag name is "noscript", if the scripting flag is enabled
             *
             * The scripting flag is never enabled in this parser.
             */
            Op::TagPush(TagName::NOEMBED) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "select"
             */
            Op::TagPush(TagName::SELECT) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.frameset_ok = false;

                match self.state.insertion_mode {
                    /*
                     * > If the insertion mode is one of "in table", "in caption", "in table body", "in row",
                     * > or "in cell", then switch the insertion mode to "in select in table".
                     */
                    InsertionMode::IN_TABLE
                    | InsertionMode::IN_CAPTION
                    | InsertionMode::IN_TABLE_BODY
                    | InsertionMode::IN_ROW
                    | InsertionMode::IN_CELL => {
                        self.state.insertion_mode = InsertionMode::IN_SELECT_IN_TABLE;
                    }

                    /*
                     * > Otherwise, switch the insertion mode to "in select".
                     */
                    _ => {
                        self.state.insertion_mode = InsertionMode::IN_SELECT;
                    }
                }
                true
            }

            /*
             * > A start tag whose tag name is one of: "optgroup", "option"
             */
            Op::TagPush(TagName::OPTGROUP | TagName::OPTION) => {
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&TagName::OPTION)
                {
                    self.state.stack_of_open_elements.pop();
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is one of: "rb", "rtc"
             */
            Op::TagPush(TagName::RB | TagName::RTC) => {
                if self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::RUBY)
                {
                    self.generate_implied_end_tags(None);

                    if self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&TagName::RUBY)
                    {
                        // @todo Indicate a parse error once it's possible.
                    }
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is one of: "rp", "rt"
             */
            Op::TagPush(TagName::RP | TagName::RT) => {
                if self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::RUBY)
                {
                    self.generate_implied_end_tags(Some(TagName::RTC));

                    let current_node_name = self
                        .state
                        .stack_of_open_elements
                        .current_node()
                        .unwrap()
                        .node_name;
                    if matches!(
                        current_node_name,
                        NodeName::Tag(TagName::RUBY | TagName::RTC)
                    ) {
                        // @todo Indicate a parse error once it's possible.
                    }
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }
        }
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

    fn step_in_table_text(&mut self) -> bool {
        todo!()
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

    ///
    /// Internal helpers
    ///

    /// Creates a new bookmark for the currently-matched token and returns the generated name.
    ///
    /// @throws Exception When unable to allocate requested bookmark.
    ///
    /// @return string|false Name of created bookmark, or false if unable to create.
    fn bookmark_token(&mut self) -> Result<Rc<str>, HtmlProcessorError> {
        let bookmark = format!("{}", self.bookmark_counter + 1);
        self.tag_processor
            .set_bookmark(&bookmark)
            .map(|_| {
                self.bookmark_counter += 1;
                bookmark.into()
            })
            .map_err(|_| HtmlProcessorError::ExceededMaxBookmarks)
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

    pub fn get_tag(&self) -> Option<TagName> {
        if self.last_error.is_some() {
            return None;
        }

        if self.is_virtual() {
            let node_name = &self.current_element.as_ref().unwrap().token.node_name;
            debug_assert!(matches!(node_name, NodeName::Tag(_)));
            return match node_name {
                NodeName::Tag(tag_name) => Some(tag_name.clone()),
                _ => unreachable!(),
            };
        }

        /*
         * > A start tag whose tag name is "image"
         * > Change the token's tag name to "img" and reprocess it. (Don't ask.)
         */
        let option_tag_name = self.tag_processor.get_tag();
        if let Some(tag_name) = &option_tag_name {
            if self.get_namespace() == ParsingNamespace::Html
                && matches!(tag_name , TagName::Arbitrary(arbitrary_name) if &**arbitrary_name == b"IMAGE")
            {
                return Some(TagName::IMG);
            }
        }
        option_tag_name
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

    pub fn get_token_name(&self) -> Option<NodeName> {
        if self.is_virtual() {
            Some(
                self.current_element
                    .as_ref()
                    .unwrap()
                    .token
                    .node_name
                    .clone(),
            )
        } else {
            self.tag_processor.get_token_name()
        }
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
            match &self.current_element.as_ref().unwrap().token.node_name {
                NodeName::Tag(_) => Some(TokenType::Tag),
                NodeName::Token(token_type) => Some(token_type.clone()),
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

    pub fn get_attribute(&self, name: &str) -> AttributeValue {
        if self.is_virtual() {
            AttributeValue::BooleanFalse
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

    pub fn get_attribute_names_with_prefix(&self, prefix: &str) -> Option<Vec<Rc<str>>> {
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
    pub fn get_modifiable_text(&self) -> Rc<str> {
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
    /// @return WP_HTML_Token|null The adjusted current node.
    fn get_adjusted_current_node(&self) -> Option<&HTMLToken> {
        if self.context_node.is_some() && self.state.stack_of_open_elements.count() == 1 {
            self.context_node.as_ref()
        } else {
            self.state.stack_of_open_elements.current_node()
        }
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
    fn insert_virtual_node(
        &mut self,
        token_name: TagName,
        bookmark_name: Option<&str>,
    ) -> HTMLToken {
        let current_bookmark = self
            .state
            .current_token
            .clone()
            .unwrap()
            .bookmark_name
            .unwrap();
        let start = {
            let here = self.tag_processor.bookmarks.get(&current_bookmark).unwrap();
            here.start
        };
        let name = self.bookmark_token().unwrap();
        self.tag_processor
            .bookmarks
            .insert(name.clone(), HtmlSpan::new(start, 0));
        let token = HTMLToken::new(Some(name.as_ref()), token_name.into(), false);
        self.insert_html_element(token.clone());
        token
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
    fn get_encoding(label: &str) -> Option<Rc<str>> {
        todo!()
    }

    fn make_op(&self) -> Op {
        match self.get_token_name() {
            Some(NodeName::Tag(tag_name)) => {
                if self.is_tag_closer() {
                    Op::TagPop(tag_name)
                } else {
                    Op::TagPush(tag_name)
                }
            }
            Some(NodeName::Token(token_type)) => Op::Token(token_type),
            None => unreachable!("Op should never be made when no token is available."),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum HtmlProcessorError {
    ExceededMaxBookmarks,
    UnsupportedException(UnsupportedException),
}
#[derive(Clone, Debug)]
pub(crate) enum UnsupportedException {}

#[derive(Debug, PartialEq)]
enum Op {
    TagPush(TagName),
    TagPop(TagName),
    Token(TokenType),
}
