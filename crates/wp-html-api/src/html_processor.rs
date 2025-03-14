#![allow(dead_code, unused_variables, non_camel_case_types)]

mod active_formatting_elements;
pub mod errors;
mod html_stack_event;
mod html_token;
mod insertion_mode;
mod processor_state;
mod stack_of_open_elements;

use std::{collections::VecDeque, rc::Rc};

use crate::{
    attributes::qualified_attribute_name,
    compat_mode::CompatMode,
    doctype::HtmlDoctypeInfo,
    tag_name::TagName,
    tag_processor::{
        AttributeValue, BookmarkName, CommentType, HtmlSpan, NodeName, ParserState,
        ParsingNamespace, TagProcessor, TextNodeClassification, TokenType,
    },
};
use active_formatting_elements::*;
use errors::{HtmlProcessorError, UnsupportedException};
use html_stack_event::*;
use html_token::*;
use insertion_mode::InsertionMode;
use processor_state::ProcessorState;
use stack_of_open_elements::StackOfOpenElements;

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

#[derive(PartialEq)]
enum EncodingConfidence {
    Tentative,
    Certain,
    Irrelevant,
}

pub struct HtmlProcessor {
    pub tag_processor: TagProcessor,
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
    pub fn create_fragment(html: &[u8], context: &str, encoding: &str) -> Option<Self> {
        if "<body>" != context {
            return None;
        }

        if "UTF-8" != encoding {
            return None;
        }

        let processor = {
            let mut context_processor = {
                let prepared_context = format!("<!DOCTYPE html>{}", context).into_bytes();
                Self::create_full_parser(&prepared_context, encoding)
            }?;

            while context_processor.next_tag(None) {
                if !context_processor.is_virtual() {
                    context_processor.set_bookmark("final_node").ok()?;
                }
            }

            if !context_processor.has_bookmark("final_node")
                || !context_processor.seek("final_node")
            {
                // @todo: _doing_it_wrong( __METHOD__, __( 'No valid context element was detected.' ), '6.8.0' );
                return None;
            }

            context_processor.create_fragment_at_current_node(html)
        };
        processor
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
    /// @param string $html Input HTML fragment to process.
    /// @return static|null The created processor if successful, otherwise null.
    fn create_fragment_at_current_node(&self, html: &[u8]) -> Option<Self> {
        if self.get_token_type() != Some(&TokenType::Tag) || self.is_tag_closer() {
            // @todo _doing_it_wrong( __METHOD__, __( 'The context element must be a start tag.' ), '6.8.0');
            return None;
        }

        let (tag_name, namespace) = {
            let ce = &self.current_element.as_ref().unwrap().token;
            let tag = ce.node_name.tag()?;
            (tag.clone(), ce.namespace.clone())
        };

        if namespace == ParsingNamespace::Html && Self::is_void(&tag_name) {
            // @todo _doing_it_wrong( __METHOD__, __( 'The context element cannot be a void element, found "%s".' ), tag_name );
            return None;
        }

        /*
         * Prevent creating fragments at nodes that require a special tokenizer state.
         * This is unsupported by the HTML Processor.
         */
        if namespace == ParsingNamespace::Html
            && matches!(
                tag_name,
                TagName::IFRAME
                    | TagName::NOEMBED
                    | TagName::NOFRAMES
                    | TagName::SCRIPT
                    | TagName::STYLE
                    | TagName::TEXTAREA
                    | TagName::TITLE
                    | TagName::XMP
                    | TagName::PLAINTEXT
            )
        {
            // @todo _doing_it_wrong( __METHOD__, __( 'The context element "%s" is not supported.' ), tag_name );
            return None;
        }

        let mut fragment_processor = Self::new(html);

        fragment_processor.tag_processor.compat_mode = self.tag_processor.compat_mode.clone();

        // @todo Create "fake" bookmarks for non-existent but implied nodes.
        fragment_processor
            .tag_processor
            .internal_bookmarks
            .insert(fragment_processor.bookmark_counter, HtmlSpan::new(0, 0));

        let root_node = HTMLToken {
            is_root_node: true,
            bookmark_name: Some(fragment_processor.bookmark_counter),
            node_name: NodeName::Tag(TagName::HTML),
            ..Default::default()
        };

        fragment_processor.bookmark_counter += 1;
        fragment_processor.push(root_node);

        fragment_processor
            .tag_processor
            .internal_bookmarks
            .insert(fragment_processor.bookmark_counter, HtmlSpan::new(0, 0));
        fragment_processor.context_node =
            Some(self.current_element.as_ref().unwrap().token.clone());
        fragment_processor
            .context_node
            .as_mut()
            .unwrap()
            .bookmark_name = Some(fragment_processor.bookmark_counter);
        fragment_processor.bookmark_counter += 1;

        if tag_name == TagName::TEMPLATE {
            fragment_processor
                .state
                .stack_of_template_insertion_modes
                .push(InsertionMode::IN_TEMPLATE);
        }

        fragment_processor.breadcrumbs =
            vec![NodeName::Tag(TagName::HTML), NodeName::Tag(tag_name)];

        fragment_processor.reset_insertion_mode_appropriately();

        /*
         * > Set the parser's form element pointer to the nearest node to the context element that
         * > is a form element (going straight up the ancestor chain, and including the element
         * > itself, if it is a form element), if any. (If there is no such form element, the
         * > form element pointer keeps its initial value, null.)
         */
        for element in self.state.stack_of_open_elements.walk_up() {
            if element.node_name == NodeName::Tag(TagName::FORM) {
                fragment_processor.state.form_element = Some(element.clone());
                fragment_processor
                    .state
                    .form_element
                    .as_mut()
                    .unwrap()
                    .bookmark_name = None;
                break;
            }
        }

        fragment_processor.state.encoding_confidence = EncodingConfidence::Irrelevant;

        /*
         * Update the parsing namespace near the end of the process.
         * This is important so that any push/pop from the stack of open
         * elements does not change the parsing namespace.
         */
        fragment_processor.tag_processor.change_parsing_namespace(
            if self
                .current_element
                .as_ref()
                .unwrap()
                .token
                .integration_node_type
                .is_some()
            {
                ParsingNamespace::Html
            } else {
                namespace
            },
        );

        Some(fragment_processor)
    }

    /// Stops the parser and terminates its execution when encountering unsupported markup.
    ///
    /// @throws WP_HTML_Unsupported_Exception Halts execution of the parser.
    ///
    /// @param string $message Explains support is missing in order to parse the current node.
    fn bail(&mut self, error: UnsupportedException) -> bool {
        self.last_error = Some(HtmlProcessorError::UnsupportedException(error));
        false
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
    /// @see self::$unsupported_exception
    ///
    /// @return WP_HTML_Unsupported_Exception|null
    pub fn get_unsupported_exception(&self) -> Option<&UnsupportedException> {
        match &self.last_error {
            Some(HtmlProcessorError::UnsupportedException(e)) => Some(e),
            _ => None,
        }
    }

    /// Gets DOCTYPE declaration info from a DOCTYPE token.
    ///
    /// DOCTYPE tokens may appear in many places in an HTML document. In most places, they are
    /// simply ignored. The main parsing functions find the basic shape of DOCTYPE tokens but
    /// do not perform detailed parsing.
    ///
    /// This method can be called to perform a full parse of the DOCTYPE token and retrieve
    /// its information.
    ///
    /// @return WP_HTML_Doctype_Info|null The DOCTYPE declaration information or `null` if not
    ///                                   currently at a DOCTYPE node.
    pub fn get_doctype_info(&self) -> Option<HtmlDoctypeInfo> {
        self.tag_processor.get_doctype_info()
    }

    /// Finds the next tag matching the query.
    ///
    /// @todo Support matching the class name and tag name.
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
                if self.get_token_type() != Some(&TokenType::Tag) {
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
                if self.get_token_type() != Some(&TokenType::Tag) {
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
            if self.get_token_type() != Some(&TokenType::Tag) || self.is_tag_closer() {
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
        let current_element = if let Some(current_element) = &self.current_element {
            current_element
        } else {
            // There are no tokens left, so close all remaining open elements
            while self.pop().is_some() {}

            return if self.element_queue.is_empty() {
                false
            } else {
                self.next_visitable_token()
            };
        };

        let is_pop = current_element.operation == StackOperation::Pop;

        // The root node only exists in the fragment parser, and closing it
        // indicates that the parse is complete. Stop before popping it from
        // the breadcrumbs.
        if current_element.token.is_root_node {
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
        if is_pop
            && !self
                .expects_closer(Some(&current_element.token))
                .unwrap_or(false)
        {
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
                    .map(|t| t == &TokenType::Tag)
                    .unwrap_or(false)
        } else {
            self.tag_processor.is_tag_closer()
        }
    }

    /// Indicates if the currently-matched token is virtual, created by a stack operation
    /// while processing HTML, rather than a token found in the HTML text itself.
    ///
    /// @return bool Whether the current token is virtual.
    fn is_virtual(&self) -> bool {
        self.current_element
            .as_ref()
            .is_some_and(|current_element| current_element.provenance == StackProvenance::Virtual)
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
    /// @param WP_HTML_Token|null $node Optional. Node to examine, if provided.
    ///                                 Default is to examine current node.
    /// @return bool|null Whether to expect a closer for the currently-matched node,
    ///                   or `null` if not matched on any token.
    pub fn expects_closer(&self, node: Option<&HTMLToken>) -> Option<bool> {
        let (node_name, namespace, has_self_closing_flag) = if let Some(token) = node {
            (
                &token.node_name,
                &token.namespace,
                token.has_self_closing_flag,
            )
        } else {
            (
                &self.get_token_name()?,
                self.get_namespace(),
                self.has_self_closing_flag(),
            )
        };

        let result = match node_name {
            // Comments, text nodes, and other atomic tokens.
            // Doctype declarations.
            NodeName::Token(
                TokenType::Text
                | TokenType::CdataSection
                | TokenType::Comment
                | TokenType::Doctype
                | TokenType::PresumptuousTag
                | TokenType::FunkyComment,
            ) => false,

            NodeName::Token(TokenType::Tag) => unreachable!("#tag NodeName should never exist"),

            // Self-closing elements in foreign content.
            NodeName::Tag(_) if *namespace != ParsingNamespace::Html => !has_self_closing_flag,

            // Void elements.
            // Special atomic elements.
            NodeName::Tag(tag_name) => {
                if matches!(
                    tag_name,
                    TagName::IFRAME
                        | TagName::NOEMBED
                        | TagName::NOFRAMES
                        | TagName::SCRIPT
                        | TagName::STYLE
                        | TagName::TEXTAREA
                        | TagName::TITLE
                        | TagName::XMP
                ) {
                    false
                } else {
                    !Self::is_void(tag_name)
                }
            }
        };
        Some(result)
    }

    /// Steps through the HTML document and stop at the next tag, if any.
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
                if !self.expects_closer(Some(top_node)).unwrap_or(false) {
                    self.pop();
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
                    Some(bookmark),
                    token_name.clone(),
                    self.has_self_closing_flag(),
                ));
            } else {
                self.last_error = Some(HtmlProcessorError::ExceededMaxBookmarks);
                return false;
            }
        }

        let parse_in_current_insertion_mode = self.state.stack_of_open_elements.count() == 0 || {
            let adjusted_current_node = self.get_adjusted_current_node().unwrap();
            let is_closer = self.is_tag_closer();
            let is_start_tag =
                self.tag_processor.parser_state == ParserState::MatchedTag && !is_closer;

            adjusted_current_node.namespace == ParsingNamespace::Html
                || (adjusted_current_node.integration_node_type
                    == Some(IntegrationNodeType::MathML)
                    && ((is_start_tag
                        && (!matches!(
                            &token_name,
                            NodeName::Tag(TagName::MALIGNMARK | TagName::MGLYPH)
                        )))
                        || token_name == TokenType::Text.into()))
                || (adjusted_current_node.namespace == ParsingNamespace::MathML
                    && adjusted_current_node.node_name == NodeName::Tag(TagName::ANNOTATION_XML)
                    && is_start_tag
                    && token_name == NodeName::Tag(TagName::SVG))
                || (adjusted_current_node.integration_node_type == Some(IntegrationNodeType::HTML)
                    && (is_start_tag || token_name == TokenType::Text.into()))
        };

        if parse_in_current_insertion_mode {
            self.step_in_current_insertion_mode()
        } else {
            self.step_in_foreign_content()
        }

        // @todo use Results
    }

    fn step_in_current_insertion_mode(&mut self) -> bool {
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
    /// @return string[] Array of tag names representing path to matched node.
    pub fn get_breadcrumbs() {
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
    /// @param string $html Input HTML to normalize.
    ///
    /// @return string|null Normalized output, or `null` if unable to normalize.
    pub fn normalize(html: &[u8]) -> Result<String, ()> {
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
                self.step(NodeToProcess::ProcessNextNode)
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
                if let Some(doctype) = self.tag_processor.get_doctype_info() {
                    if doctype.indicated_compatability_mode == CompatMode::Quirks {
                        self.tag_processor.compat_mode = CompatMode::Quirks;
                    }
                }

                /*
                 * > Then, switch the insertion mode to "before html".
                 */
                self.state.insertion_mode = InsertionMode::BEFORE_HTML;
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }
            /*
             * > Anything else
             */
            _ => {
                self.tag_processor.compat_mode = CompatMode::Quirks;
                self.state.insertion_mode = InsertionMode::BEFORE_HTML;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
    }

    /// Parses next element in the 'before html' insertion mode.
    ///
    /// This internal function performs the 'before html' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
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
            Op::Token(TokenType::Doctype) => self.step(NodeToProcess::ProcessNextNode),

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
                if !matches!(
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
                if !matches!(
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
                if let Some(AttributeValue::String(_)) = self.get_attribute(b"charset") {
                    if EncodingConfidence::Tentative == self.state.encoding_confidence {
                        return self.bail(UnsupportedException::MetaTagCharsetDetermineEncoding);
                    }
                }

                /*
                 * >   - Otherwise, if the element has an http-equiv attribute whose value is
                 * >     an ASCII case-insensitive match for the string "Content-Type", and
                 * >     the element has a content attribute, and applying the algorithm for
                 * >     extracting a character encoding from a meta element to that attribute's
                 * >     value returns an encoding, and the confidence is currently tentative,
                 * >     then change the encoding to the extracted encoding.
                 */
                if let (Some(AttributeValue::String(http_equiv)), Some(AttributeValue::String(_))) = (
                    self.get_attribute(b"http-equiv"),
                    self.get_attribute(b"content"),
                ) {
                    if http_equiv.eq_ignore_ascii_case(b"Content-Type")
                        && self.state.encoding_confidence == EncodingConfidence::Tentative
                    {
                        return self.bail(UnsupportedException::MetaTagHttpEquivDetermineEncoding);
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
                self.pop();
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
                true
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
                    self.pop_until(&TagName::TEMPLATE);
                    self.state
                        .active_formatting_elements
                        .clear_up_to_last_marker();
                    self.state.stack_of_template_insertion_modes.pop();
                    self.reset_insertion_mode_appropriately();
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

            /*
             * > Anything else
             */
            _ => {
                self.pop();
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
                self.pop();
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
                self.pop();
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
                self.bail(UnsupportedException::AfterHeadElementsReopenHead)
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
                    self.bail(UnsupportedException::CannotProcessNonIgnoredFrameset)
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
                        self.pop();
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
                self.state.frameset_ok = false;
                let mut node = self.state.stack_of_open_elements.current_node();
                let is_li = tag_name == TagName::LI;

                /*
                 * The logic for LI and DT/DD is the same except for one point: LI elements _only_
                 * close other LI elements, but a DT or DD element closes _any_ open DT or DD element.
                 */
                loop {
                    match node {
                        Some(
                            some_node @ HTMLToken {
                                node_name: NodeName::Tag(current_tag_name),
                                ..
                            },
                        ) => {
                            let current_tag_name = current_tag_name.clone();
                            let match_tag_name_test = if is_li {
                                current_tag_name == TagName::LI
                            } else {
                                matches!(current_tag_name, TagName::DD | TagName::DT)
                            };
                            if match_tag_name_test {
                                self.generate_implied_end_tags(Some(&current_tag_name));
                                if !self
                                    .state
                                    .stack_of_open_elements
                                    .current_node_is(&NodeName::Tag(current_tag_name.clone()))
                                {
                                    // @todo Indicate a parse error once it's possible. This error does not impact the logic here.
                                }

                                self.pop_until(&current_tag_name);
                                break;
                            }

                            /*
                             * > If node is in the special category, but is not an address, div,
                             * > or p element, then jump to the step labeled done below.
                             */
                            if !matches!(
                                current_tag_name,
                                TagName::ADDRESS | TagName::DIV | TagName::P
                            ) && Self::is_special(&current_tag_name)
                            {
                                break;
                            }

                            /*
                             * > Otherwise, set node to the previous entry in the stack of open elements
                             * > and return to the step labeled loop.
                             */
                            node = self
                                .state
                                .stack_of_open_elements
                                .walk_up()
                                .skip_while(|&stack_node| stack_node != some_node)
                                .nth(1);
                            continue;
                        }
                        None => break,
                        _ => {
                            unreachable!("Should not have token nodes here")
                        }
                    }
                }
                if self.state.stack_of_open_elements.has_p_in_button_scope() {
                    self.close_a_p_element();
                }

                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
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
                self.bail(UnsupportedException::CannotProcessPlaintextElements)
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
                    self.pop_until(&TagName::BUTTON);
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
                    if !self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&NodeName::Tag(tag_name.clone()))
                    {
                        // Parse error: this error doesn't impact parsing.
                    }
                    self.pop_until(&tag_name);
                    true
                }
            }

            /*
             * > An end tag whose tag name is "form"
             */
            Op::TagPop(TagName::FORM) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .contains(&TagName::TEMPLATE)
                {
                    let node = self.state.form_element.take();

                    /*
                     * > If node is null or if the stack of open elements does not have node
                     * > in scope, then this is a parse error; return and ignore the token.
                     *
                     * @todo It's necessary to check if the form token itself is in scope, not
                     *       simply whether any FORM is in scope.
                     */
                    if node.is_none()
                        || !self
                            .state
                            .stack_of_open_elements
                            .has_element_in_scope(&TagName::FORM)
                    {
                        // Parse error: ignore the token.
                        return self.step(NodeToProcess::ProcessNextNode);
                    }

                    let node = node.unwrap();

                    self.generate_implied_end_tags(None);
                    if node != *self.state.stack_of_open_elements.current_node().unwrap() {
                        // @todo Indicate a parse error once it's possible. This error does not impact the logic here.
                        return self
                            .bail(UnsupportedException::CannotCloseFormWithOtherElementsOpen);
                    }
                    self.remove_node_from_stack_of_open_elements(&node);
                    true
                } else {
                    /*
                     * > If the stack of open elements does not have a form element in scope,
                     * > then this is a parse error; return and ignore the token.
                     *
                     * Note that unlike in the clause above, this is checking for any FORM in scope.
                     */
                    if !self
                        .state
                        .stack_of_open_elements
                        .has_element_in_scope(&TagName::FORM)
                    {
                        // Parse error: ignore the token.
                        return self.step(NodeToProcess::ProcessNextNode);
                    }

                    self.generate_implied_end_tags(None);

                    if !self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&NodeName::Tag(TagName::FORM))
                    {
                        // @todo Indicate a parse error once it's possible. This error does not impact the logic here.
                    }

                    self.pop_until(&TagName::FORM);
                    true
                }
            }

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
            Op::TagPop(tag_name @ (TagName::LI | TagName::DD | TagName::DT)) => {
                if
                /*
                 * An end tag whose tag name is "li":
                 * If the stack of open elements does not have an li element in list item scope,
                 * then this is a parse error; ignore the token.
                 */
                (
                        TagName::LI == tag_name &&
                        !self.state.stack_of_open_elements.has_element_in_list_item_scope(&TagName::LI)
                    ) ||
                    /*
                     * An end tag whose tag name is one of: "dd", "dt":
                     * If the stack of open elements does not have an element in scope that is an
                     * HTML element with the same tag name as that of the token, then this is a
                     * parse error; ignore the token.
                     */
                    (
                        TagName::LI != tag_name &&
                        !self.state.stack_of_open_elements.has_element_in_scope(&tag_name)
                    )
                {
                    /*
                     * This is a parse error, ignore the token.
                     *
                     * @todo Indicate a parse error once it's possible.
                     */
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.generate_implied_end_tags(Some(&tag_name));

                if !self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(tag_name.clone()))
                {
                    // @todo Indicate a parse error once it's possible. This error does not impact the logic here.
                }

                self.pop_until(&tag_name);
                true
            }

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

                    if !self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&NodeName::Tag(tag_name))
                    {
                        // Parse error: this error doesn't impact parsing.
                    }

                    self.pop_until_any_h1_to_h6();
                    true
                }
            }

            /*
             * > A start tag whose tag name is "a"
             */
            Op::TagPush(TagName::A) => {
                let item = self
                    .state
                    .active_formatting_elements
                    .walk_up()
                    .find(|item| {
                        matches!(
                            item,
                            ActiveFormattingElement::Marker
                                | ActiveFormattingElement::Token(HTMLToken {
                                    node_name: NodeName::Tag(TagName::A),
                                    ..
                                })
                        )
                    });
                if let Some(ActiveFormattingElement::Token(a_token)) = item {
                    let remove_token = a_token.clone();
                    self.run_adoption_agency_algorithm();
                    self.state
                        .active_formatting_elements
                        .remove_node(&remove_token);
                    self.remove_node_from_stack_of_open_elements(&remove_token);
                }

                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state
                    .active_formatting_elements
                    .push(self.state.current_token.clone().unwrap());
                true
            }

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
                    if !self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&NodeName::Tag(tag_name))
                    {
                        // This is a parse error.
                    }

                    self.pop_until(&self.get_tag().unwrap());
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
                match self.get_attribute(b"type") {
                    Some(AttributeValue::String(type_attr_value))
                        if !type_attr_value.eq_ignore_ascii_case(b"hidden") =>
                    {
                        self.state.frameset_ok = false;
                    }
                    Some(AttributeValue::String(_)) => {}
                    Some(AttributeValue::BooleanFalse | AttributeValue::BooleanTrue) => {
                        self.state.frameset_ok = false;
                    }
                    None => {}
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
                    .current_node_is(&NodeName::Tag(TagName::OPTION))
                {
                    self.pop();
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
                        .current_node_is(&NodeName::Tag(TagName::RUBY))
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
                    self.generate_implied_end_tags(Some(&TagName::RTC));

                    let current_node_name = self
                        .state
                        .stack_of_open_elements
                        .current_node()
                        .unwrap()
                        .node_name
                        .clone();
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

            /*
             * > A start tag whose tag name is "math"
             */
            Op::TagPush(TagName::MATH) => {
                self.reconstruct_active_formatting_elements();

                /*
                 * @todo Adjust MathML attributes for the token. (This fixes the case of MathML attributes that are not all lowercase.)
                 * @todo Adjust foreign attributes for the token. (This fixes the use of namespaced attributes, in particular XLink.)
                 *
                 * These ought to be handled in the attribute methods.
                 */
                let token = self.state.current_token.as_mut().unwrap();
                token.namespace = ParsingNamespace::MathML;
                let token = token.clone();
                let has_self_closing_flag = token.has_self_closing_flag;
                self.insert_html_element(token);
                if has_self_closing_flag {
                    self.pop();
                }
                true
            }

            /*
             * > A start tag whose tag name is "svg"
             */
            Op::TagPush(TagName::SVG) => {
                self.reconstruct_active_formatting_elements();

                /*
                 * @todo Adjust SVG attributes for the token. (This fixes the case of SVG attributes that are not all lowercase.)
                 * @todo Adjust foreign attributes for the token. (This fixes the use of namespaced attributes, in particular XLink in SVG.)
                 *
                 * These ought to be handled in the attribute methods.
                 */
                let token = self.state.current_token.as_mut().unwrap();
                token.namespace = ParsingNamespace::Svg;
                let token = token.clone();
                let has_self_closing_flag = token.has_self_closing_flag;
                self.insert_html_element(token);
                if has_self_closing_flag {
                    self.pop();
                }
                true
            }

            /*
             * > A start tag whose tag name is one of: "caption", "col", "colgroup",
             * > "frame", "head", "tbody", "td", "tfoot", "th", "thead", "tr"
             *
            	* Parse error. Ignore the token.
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::FRAME
                | TagName::HEAD
                | TagName::TBODY
                | TagName::TD
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TR,
            ) => self.step(NodeToProcess::ProcessNextNode),

            /*
             * > Any other start tag
             */
            Op::TagPush(_) => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > Any other end tag
             *
             * @todo this can probably be refactored to pop N times with an enumerate instead of
             * cloning the token.
             */
            Op::TagPop(tag_name) => {
                // > Run these steps:
                // >   1. Initialize node to be the current node (the bottommost node of the stack).
                // >   2. Loop: If node is an HTML element with the same tag name as the token, then:
                // >      1. Generate implied end tags, except for HTML elements with the same tag name as the token.
                // >      2. If node is not the current node, then this is a parse error.
                // >      3. Pop all the nodes from the current node up to node, including node, then stop these steps.
                // >   3. Otherwise, if node is in the special category, then this is a parse error; ignore the token, and return.
                // >   4. Set node to the previous entry in the stack of open elements.
                // >   5. Return to the step labeled loop.

                enum Continuation {
                    FoundSpecial,
                    FoundMatchingNode,
                }

                let continuation: Option<Continuation> = self
                    .state
                    .stack_of_open_elements
                    .walk_up()
                    .find_map(|node| {
                        if node.namespace != ParsingNamespace::Html {
                            return None;
                        }

                        let node_tag_name = node.node_name.tag()?;

                        if *node_tag_name == tag_name {
                            return Some(Continuation::FoundMatchingNode);
                        }

                        if Self::is_special(node_tag_name) {
                            return Some(Continuation::FoundSpecial);
                        }

                        None
                    });

                match continuation {
                    Some(Continuation::FoundSpecial) => {
                        // This is a parse error, ignore the token.
                        self.step(NodeToProcess::ProcessNextNode)
                    }
                    Some(Continuation::FoundMatchingNode) => {
                        self.generate_implied_end_tags(Some(&tag_name));

                        // @todo "If node is not the current node, then this is a parse error."
                        self.pop_until(&tag_name);
                        true
                    }
                    None => false,
                }
            }

            Op::Token(TokenType::CdataSection) => unreachable!("CDATA does not exist in HTML5."),
            Op::Token(TokenType::Tag) => {
                unreachable!("#tag token operations are handled by Op::TagPush and Op::TagPop.")
            }
        }
    }

    /// Parses next element in the 'in table' insertion mode.
    ///
    /// This internal function performs the 'in table' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intable
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_table(&mut self) -> bool {
        let HTMLToken {
            node_name: current_node_tag_name,
            ..
        } = self
            .state
            .stack_of_open_elements
            .current_node()
            .expect("Step in table expects a current node.");
        let current_node_tag_name = match current_node_tag_name {
            NodeName::Tag(tag_name) => tag_name,
            NodeName::Token(_) => {
                unreachable!("Step in table should never have a non-tag current node.")
            }
        };

        match self.make_op() {
            /*
             * > A character token, if the current node is table,
             * > tbody, template, tfoot, thead, or tr element
             */
            Op::Token(TokenType::Text)
                if matches!(
                    current_node_tag_name,
                    TagName::TABLE
                        | TagName::TBODY
                        | TagName::TEMPLATE
                        | TagName::TFOOT
                        | TagName::THEAD
                        | TagName::TR
                ) =>
            {
                match self.tag_processor.text_node_classification {
                    /*
                     * If the text is empty after processing HTML entities and stripping
                     * U+0000 NULL bytes then ignore the token.
                     */
                    TextNodeClassification::NullSequence => {
                        self.step(NodeToProcess::ProcessNextNode)
                    }
                    /*
                     * This follows the rules for "in table text" insertion mode.
                     *
                     * Whitespace-only text nodes are inserted in-place. Otherwise
                     * foster parenting is enabled and the nodes would be
                     * inserted out-of-place.
                     *
                     * > If any of the tokens in the pending table character tokens
                     * > list are character tokens that are not ASCII whitespace,
                     * > then this is a parse error: reprocess the character tokens
                     * > in the pending table character tokens list using the rules
                     * > given in the "anything else" entry in the "in table"
                     * > insertion mode.
                     * >
                     * > Otherwise, insert the characters given by the pending table
                     * > character tokens list.
                     *
                     * @see https://html.spec.whatwg.org/#parsing-main-intabletext
                     */
                    TextNodeClassification::Whitespace => {
                        self.insert_html_element(self.state.current_token.clone().unwrap());
                        true
                    }

                    // Non-whitespace would trigger fostering, unsupported at this time.
                    TextNodeClassification::Generic => {
                        self.bail(UnsupportedException::FosterParenting)
                    }
                }
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
             * > A start tag whose tag name is "caption"
             */
            Op::TagPush(TagName::CAPTION) => {
                self.clear_to_table_context();
                self.state.active_formatting_elements.insert_marker();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_CAPTION;
                true
            }

            /*
             * > A start tag whose tag name is "colgroup"
             */
            Op::TagPush(TagName::COLGROUP) => {
                self.clear_to_table_context();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_COLUMN_GROUP;
                true
            }

            /*
             * > A start tag whose tag name is "col"
             */
            Op::TagPush(TagName::COL) => {
                self.clear_to_table_context();

                /*
                 * > Insert an HTML element for a "colgroup" start tag token with no attributes,
                 * > then switch the insertion mode to "in column group".
                 */
                self.insert_virtual_node(TagName::COLGROUP, None);
                self.state.insertion_mode = InsertionMode::IN_COLUMN_GROUP;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > A start tag whose tag name is one of: "tbody", "tfoot", "thead"
             */
            Op::TagPush(TagName::TBODY | TagName::TFOOT | TagName::THEAD) => {
                self.clear_to_table_context();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                true
            }

            /*
             * > A start tag whose tag name is one of: "td", "th", "tr"
             */
            Op::TagPush(TagName::TD | TagName::TH | TagName::TR) => {
                self.clear_to_table_context();

                /*
                 * > Insert an HTML element for a "tbody" start tag token with no attributes,
                 * > then switch the insertion mode to "in table body".
                 */
                self.insert_virtual_node(TagName::TBODY, None);
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > A start tag whose tag name is "table"
             *
             * This tag in the IN TABLE insertion mode is a parse error.
             */
            Op::TagPush(TagName::TABLE) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TABLE)
                {
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop_until(&TagName::TABLE);
                    self.reset_insertion_mode_appropriately();
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }

            /*
             * > An end tag whose tag name is "table"
             */
            Op::TagPop(TagName::TABLE) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TABLE)
                {
                    // @todo Indicate a parse error once it's possible.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop_until(&TagName::TABLE);
                    self.reset_insertion_mode_appropriately();
                    true
                }
            }

            /*
             * > An end tag whose tag name is one of: "body", "caption", "col", "colgroup", "html", "tbody", "td", "tfoot", "th", "thead", "tr"
             *
            	* Parse error: ignore the token.
             */
            Op::TagPop(
                TagName::BODY
                | TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::HTML
                | TagName::TBODY
                | TagName::TD
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TR,
            ) => self.step(NodeToProcess::ProcessNextNode),

            /*
             * > A start tag whose tag name is one of: "style", "script", "template"
             * > An end tag whose tag name is "template"
             *
             *   > Process the token using the rules for the "in head" insertion mode.
             */
            Op::TagPush(TagName::STYLE | TagName::SCRIPT | TagName::TEMPLATE)
            | Op::TagPop(TagName::TEMPLATE) => self.step_in_head(),

            /*
             * > A start tag whose tag name is "input"
             *
             * > If the token does not have an attribute with the name "type", or if it does, but
             * > that attribute's value is not an ASCII case-insensitive match for the string
             * > "hidden", then: act as described in the "anything else" entry below.
             */
            Op::TagPush(TagName::INPUT)
                if self
                    .get_attribute(b"type")
                    .is_some_and(|type_value| match type_value {
                        AttributeValue::String(type_value) => {
                            type_value.eq_ignore_ascii_case(b"hidden")
                        }
                        _ => false,
                    }) =>
            {
                // @todo Indicate a parse error once it's possible.
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "form"
             *
             * This tag in the IN TABLE insertion mode is a parse error.
             */
            Op::TagPush(TagName::FORM) => {
                if self
                    .state
                    .stack_of_open_elements
                    .has_element_in_scope(&TagName::TEMPLATE)
                    || self.state.form_element.is_some()
                {
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    // This FORM is special because it immediately closes and cannot have other children.
                    self.insert_html_element(self.state.current_token.clone().unwrap());
                    self.state.form_element = Some(self.state.current_token.clone().unwrap());
                    self.pop();
                    true
                }
            }

            /*
             * > Anything else
             * > Parse error. Enable foster parenting, process the token using the rules for the
             * > "in body" insertion mode, and then disable foster parenting.
             *
             * @todo Indicate a parse error once it's possible.
             */
            _ => self.bail(UnsupportedException::FosterParenting),
        }
    }

    /// Parses next element in the 'in table text' insertion mode.
    ///
    /// This internal function performs the 'in table text' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
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
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incaption
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_caption(&mut self) -> bool {
        match self.make_op() {
            /*
             * > An end tag whose tag name is "caption"
             * > A start tag whose tag name is one of: "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr"
             * > An end tag whose tag name is "table"
             *
             * These tag handling rules are identical except for the final instruction.
             * Handle them in a single block.
             */
            op @ (Op::TagPop(TagName::CAPTION)
            | Op::TagPush(
                TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::TBODY
                | TagName::TD
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TR,
            )
            | Op::TagPop(TagName::TABLE)) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::CAPTION)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.generate_implied_end_tags(None);
                if !self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::CAPTION))
                {
                    // @todo Indicate a parse error once it's possible.
                }

                self.pop_until(&TagName::CAPTION);
                self.state
                    .active_formatting_elements
                    .clear_up_to_last_marker();
                self.state.insertion_mode = InsertionMode::IN_TABLE;

                // If this is not a CAPTION end tag, the token should be reprocessed.
                if op != Op::TagPop(TagName::CAPTION) {
                    self.step(NodeToProcess::ReprocessCurrentNode)
                } else {
                    true
                }
            }

            /*
             * > An end tag whose tag name is one of: "body", "col", "colgroup", "html", "tbody", "td", "tfoot", "th", "thead", "tr"
             */
            Op::TagPop(
                TagName::BODY
                | TagName::COL
                | TagName::COLGROUP
                | TagName::HTML
                | TagName::TBODY
                | TagName::TD
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TR,
            ) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             * >   Process the token using the rules for the "in body" insertion mode.
             */
            _ => self.step_in_body(),
        }
    }

    /// Parses next element in the 'in column group' insertion mode.
    ///
    /// This internal function performs the 'in column group' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incolgroup
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_column_group(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * > U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
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
                // @todo Indicate a parse error once it's possible.
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "html"
             */
            Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > A start tag whose tag name is "col"
             */
            Op::TagPush(TagName::COL) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.pop();
                true
            }

            /*
             * > An end tag whose tag name is "colgroup"
             */
            Op::TagPop(TagName::COLGROUP) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::COLGROUP))
                {
                    // @todo Indicate a parse error once it's possible.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop();
                    self.state.insertion_mode = InsertionMode::IN_TABLE;
                    true
                }
            }

            /*
             * > An end tag whose tag name is "col"
             */
            Op::TagPop(TagName::COL) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "template"
             * > An end tag whose tag name is "template"
             */
            Op::TagPush(TagName::TEMPLATE) | Op::TagPop(TagName::TEMPLATE) => self.step_in_head(),

            /*
             * > Anything else
             */
            _ => {
                if !self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::COLGROUP))
                {
                    // @todo Indicate a parse error once it's possible.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop();
                    self.state.insertion_mode = InsertionMode::IN_TABLE;
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }
        }
    }

    /// Parses next element in the 'in table body' insertion mode.
    ///
    /// This internal function performs the 'in table body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intbody
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_table_body(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A start tag whose tag name is "tr"
             */
            Op::TagPush(TagName::TR) => {
                self.clear_to_table_body_context();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_ROW;
                true
            }

            /*
             * > A start tag whose tag name is one of: "th", "td"
             */
            Op::TagPush(TagName::TH | TagName::TD) => {
                // @todo Indicate a parse error once it's possible.
                self.clear_to_table_body_context();
                self.insert_virtual_node(TagName::TR, None);
                self.state.insertion_mode = InsertionMode::IN_ROW;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > An end tag whose tag name is one of: "tbody", "tfoot", "thead"
             */
            Op::TagPop(tag_name @ (TagName::TBODY | TagName::TFOOT | TagName::THEAD)) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.clear_to_table_body_context();
                    self.pop();
                    self.state.insertion_mode = InsertionMode::IN_TABLE;
                    true
                }
            }

            /*
             * > A start tag whose tag name is one of: "caption", "col", "colgroup", "tbody", "tfoot", "thead"
             * > An end tag whose tag name is "table"
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD,
            )
            | Op::TagPop(TagName::TABLE) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TBODY)
                    && !self
                        .state
                        .stack_of_open_elements
                        .has_element_in_table_scope(&TagName::THEAD)
                    && !self
                        .state
                        .stack_of_open_elements
                        .has_element_in_table_scope(&TagName::TFOOT)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.clear_to_table_body_context();
                    self.pop();
                    self.state.insertion_mode = InsertionMode::IN_TABLE;
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }

            /*
             * > An end tag whose tag name is one of: "body", "caption", "col", "colgroup", "html", "td", "th", "tr"
             */
            Op::TagPop(
                TagName::BODY
                | TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::HTML
                | TagName::TD
                | TagName::TH
                | TagName::TR,
            ) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             * > Process the token using the rules for the "in table" insertion mode.
             */
            _ => self.step_in_table(),
        }
    }

    /// Parses next element in the 'in row' insertion mode.
    ///
    /// This internal function performs the 'in row' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intr
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_row(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A start tag whose tag name is one of: "th", "td"
             */
            Op::TagPush(TagName::TH | TagName::TD) => {
                self.clear_to_table_row_context();
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.state.insertion_mode = InsertionMode::IN_CELL;
                self.state.active_formatting_elements.insert_marker();
                true
            }

            /*
             * > An end tag whose tag name is "tr"
             */
            Op::TagPop(TagName::TR) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TR)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.clear_to_table_row_context();
                self.pop();
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                true
            }

            /*
             * > A start tag whose tag name is one of: "caption", "col", "colgroup", "tbody", "tfoot", "thead", "tr"
             * > An end tag whose tag name is "table"
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD
                | TagName::TR,
            )
            | Op::TagPop(TagName::TABLE) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TR)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.clear_to_table_row_context();
                self.pop();
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > An end tag whose tag name is one of: "tbody", "tfoot", "thead"
             */
            Op::TagPop(tag_name @ (TagName::TBODY | TagName::TFOOT | TagName::THEAD)) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&TagName::TR)
                {
                    // Ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.clear_to_table_row_context();
                self.pop();
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > An end tag whose tag name is one of: "body", "caption", "col", "colgroup", "html", "td", "th"
             */
            Op::TagPop(
                TagName::BODY
                | TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::HTML
                | TagName::TD
                | TagName::TH,
            ) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Anything else
             * >   Process the token using the rules for the "in table" insertion mode.
             */
            _ => self.step_in_table(),
        }
    }

    /// Parses next element in the 'in cell' insertion mode.
    ///
    /// This internal function performs the 'in cell' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intd
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_cell(&mut self) -> bool {
        match self.make_op() {
            /*
             * > An end tag whose tag name is one of: "td", "th"
             */
            Op::TagPop(tag_name @ (TagName::TD | TagName::TH)) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                self.generate_implied_end_tags(None);

                /*
                 * @todo This needs to check if the current node is an HTML element, meaning that
                 *       when SVG and MathML support is added, this needs to differentiate between an
                 *       HTML element of the given name, such as `<center>`, and a foreign element of
                 *       the same given name.
                 */
                if !self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(tag_name.clone()))
                {
                    // @todo Indicate a parse error once it's possible.
                }

                self.pop_until(&tag_name);
                self.state
                    .active_formatting_elements
                    .clear_up_to_last_marker();
                self.state.insertion_mode = InsertionMode::IN_ROW;
                true
            }

            /*
             * > A start tag whose tag name is one of: "caption", "col", "colgroup", "tbody", "td",
             * > "tfoot", "th", "thead", "tr"
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::COL
                | TagName::COLGROUP
                | TagName::TBODY
                | TagName::TD
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TR,
            ) => {
                /*
                 * > Assert: The stack of open elements has a td or th element in table scope.
                 *
                 * Nothing to do here, except to verify in tests that this never appears.
                 */

                self.close_cell();
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > An end tag whose tag name is one of: "body", "caption", "col", "colgroup", "html"
             */
            Op::TagPop(
                TagName::BODY | TagName::CAPTION | TagName::COL | TagName::COLGROUP | TagName::HTML,
            ) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > An end tag whose tag name is one of: "table", "tbody", "tfoot", "thead", "tr"
             */
            Op::TagPop(
                tag_name @ (TagName::TABLE
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD
                | TagName::TR),
            ) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&tag_name)
                {
                    // Parse error: ignore the token.
                    return self.step(NodeToProcess::ProcessNextNode);
                }
                self.close_cell();
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > Anything else
             * >   Process the token using the rules for the "in body" insertion mode.
             */
            _ => self.step_in_body(),
        }
    }

    /// Parses next element in the 'in select' insertion mode.
    ///
    /// This internal function performs the 'in select' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inselect
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_select(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is U+0000 NULL
             *
             * If a text node only comprises null bytes then it should be
             * entirely ignored and should not return to calling code.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::NullSequence =>
            {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > Any other character token
             */
            Op::Token(TokenType::Text) => {
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
             * > A start tag whose tag name is "option"
             */
            Op::TagPush(TagName::OPTION) => {
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTION))
                {
                    self.pop();
                }
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > A start tag whose tag name is "optgroup"
             * > A start tag whose tag name is "hr"
             *
             * These rules are identical except for the treatment of the self-closing flag and
             * the subsequent pop of the HR void element, all of which is handled elsewhere in the processor.
             */
            Op::TagPush(TagName::OPTGROUP | TagName::HR) => {
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTION))
                {
                    self.pop();
                }
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTGROUP))
                {
                    self.pop();
                }
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > An end tag whose tag name is "optgroup"
             */
            Op::TagPop(TagName::OPTGROUP) => {
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTION))
                {
                    // Get the current node parent
                    if let Some(parent) = {
                        let mut walker = self.state.stack_of_open_elements.walk_up();
                        walker.next();
                        walker.next()
                    } {
                        if parent.node_name.tag() == Some(&TagName::OPTGROUP) {
                            self.pop();
                        }
                    }
                }

                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTGROUP))
                {
                    self.pop();
                    return true;
                }

                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > An end tag whose tag name is "option"
             */
            Op::TagPop(TagName::OPTION) => {
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::OPTION))
                {
                    self.pop();
                    true
                } else {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                }
            }

            /*
             * > An end tag whose tag name is "select"
             * > A start tag whose tag name is "select"
             *
             * It just gets treated like an end tag.
             */
            Op::TagPop(TagName::SELECT) | Op::TagPush(TagName::SELECT) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_select_scope(&TagName::SELECT)
                {
                    // Parse error: ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop_until(&TagName::SELECT);
                    self.reset_insertion_mode_appropriately();
                    true
                }
            }
            /*
             * > A start tag whose tag name is one of: "input", "keygen", "textarea"
             *
             * All three of these tags are considered a parse error when found in this insertion mode.
             */
            Op::TagPush(TagName::INPUT | TagName::KEYGEN | TagName::TEXTAREA) => {
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_select_scope(&TagName::SELECT)
                {
                    // Ignore the token.
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop_until(&TagName::SELECT);
                    self.reset_insertion_mode_appropriately();
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }

            /*
             * > A start tag whose tag name is one of: "script", "template"
             * > An end tag whose tag name is "template"
             */
            Op::TagPush(TagName::SCRIPT | TagName::TEMPLATE) | Op::TagPop(TagName::TEMPLATE) => {
                self.step_in_head()
            }

            /*
             * > Anything else
             * >   Parse error: ignore the token.
             */
            _ => self.step(NodeToProcess::ProcessNextNode),
        }
    }

    /// Parses next element in the 'in select in table' insertion mode.
    ///
    /// This internal function performs the 'in select in table' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselectintable
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_select_in_table(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A start tag whose tag name is one of: "caption", "table", "tbody", "tfoot", "thead", "tr", "td", "th"
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::TABLE
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD
                | TagName::TR
                | TagName::TD
                | TagName::TH,
            ) => {
                // @todo Indicate a parse error once it's possible.
                self.pop_until(&TagName::SELECT);
                self.reset_insertion_mode_appropriately();
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > An end tag whose tag name is one of: "caption", "table", "tbody", "tfoot", "thead", "tr", "td", "th"
             */
            Op::TagPop(
                tag_name @ (TagName::CAPTION
                | TagName::TABLE
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD
                | TagName::TR
                | TagName::TD
                | TagName::TH),
            ) => {
                // @todo Indicate a parse error once it's possible.
                if !self
                    .state
                    .stack_of_open_elements
                    .has_element_in_table_scope(&tag_name)
                {
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.pop_until(&TagName::SELECT);
                    self.reset_insertion_mode_appropriately();
                    self.step(NodeToProcess::ReprocessCurrentNode)
                }
            }

            /*
             * > Anything else
             */
            _ => self.step_in_select(),
        }
    }

    /// Parses next element in the 'in template' insertion mode.
    ///
    /// This internal function performs the 'in template' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intemplate
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_template(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token
             * > A comment token
             * > A DOCTYPE token
             */
            Op::Token(
                TokenType::Text
                | TokenType::Comment
                | TokenType::FunkyComment
                | TokenType::PresumptuousTag
                | TokenType::Doctype,
            ) => self.step_in_body(),

            /*
             * > A start tag whose tag name is one of: "base", "basefont", "bgsound", "link",
             * > "meta", "noframes", "script", "style", "template", "title"
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
             * > A start tag whose tag name is one of: "caption", "colgroup", "tbody", "tfoot", "thead"
             */
            Op::TagPush(
                TagName::CAPTION
                | TagName::COLGROUP
                | TagName::TBODY
                | TagName::TFOOT
                | TagName::THEAD,
            ) => {
                self.state.stack_of_template_insertion_modes.pop();
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_TABLE);
                self.state.insertion_mode = InsertionMode::IN_TABLE;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > A start tag whose tag name is "col"
             */
            Op::TagPush(TagName::COL) => {
                self.state.stack_of_template_insertion_modes.pop();
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_COLUMN_GROUP);
                self.state.insertion_mode = InsertionMode::IN_COLUMN_GROUP;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > A start tag whose tag name is "tr"
             */
            Op::TagPush(TagName::TR) => {
                self.state.stack_of_template_insertion_modes.pop();
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_TABLE_BODY);
                self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > A start tag whose tag name is one of: "td", "th"
             */
            Op::TagPush(TagName::TD | TagName::TH) => {
                self.state.stack_of_template_insertion_modes.pop();
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_ROW);
                self.state.insertion_mode = InsertionMode::IN_ROW;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > Any other start tag
             */
            Op::TagPush(_) => {
                self.state.stack_of_template_insertion_modes.pop();
                self.state
                    .stack_of_template_insertion_modes
                    .push(InsertionMode::IN_BODY);
                self.state.insertion_mode = InsertionMode::IN_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }

            /*
             * > Any other end tag
             * >   Parse error: ignore the token.
             */
            Op::TagPop(_) => self.step(NodeToProcess::ProcessNextNode),

            Op::Token(TokenType::CdataSection) => {
                unreachable!("CDATA sections cannot appear in HTML content.")
            }
            Op::Token(TokenType::Tag) => {
                unreachable!(
                    "TAG tokens are represented as Op::TagPush or Op::TagPop, never Op::Token."
                )
            }
        }
    }

    /// Parses next element in the 'after body' insertion mode.
    ///
    /// This internal function performs the 'after body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterbody
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_after_body(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * >   U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             *
             * > Process the token using the rules for the "in body" insertion mode.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_body()
            }

            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => self.bail(UnsupportedException::ContentOutsideOfBody),

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
             * > An end tag whose tag name is "html"
             *
             * > If the parser was created as part of the HTML fragment parsing algorithm,
             * > this is a parse error; ignore the token. (fragment case)
             * >
             * > Otherwise, switch the insertion mode to "after after body".
             */
            Op::TagPop(TagName::HTML) => {
                if self.context_node.is_some() {
                    self.step(NodeToProcess::ProcessNextNode)
                } else {
                    self.state.insertion_mode = InsertionMode::AFTER_AFTER_BODY;
                    /*
                     * The HTML element is not removed from the stack of open elements.
                     * Only internal state has changed, this does not qualify as a "step"
                     * in terms of advancing through the document to another token.
                     * Nothing has been pushed or popped.
                     * Proceed to parse the next item.
                     */
                    self.step(NodeToProcess::ProcessNextNode)
                }
            }

            _ => {
                /*
                 * > Anything else
                 *   > Parse error. Switch the insertion mode to "in body" and reprocess the token.
                 */
                self.state.insertion_mode = InsertionMode::IN_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
    }

    /// Parses next element in the 'in frameset' insertion mode.
    ///
    /// This internal function performs the 'in frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inframeset
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_frameset(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * >   U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             * >
             * > Insert the character.
             *
             * This algorithm effectively strips non-whitespace characters from text and inserts
             * them under HTML. This is not supported at this time.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_body()
            }
            Op::Token(TokenType::Text) => {
                self.bail(UnsupportedException::NonWhitespaceTextInFrameset)
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
             * > A start tag whose tag name is "frameset"
             */
            Op::TagPush(TagName::FRAMESET) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                true
            }

            /*
             * > An end tag whose tag name is "frameset"
             */
            Op::TagPop(TagName::FRAMESET) => {
                /*
                 * > If the current node is the root html element, then this is a parse error;
                 * > ignore the token. (fragment case)
                 */
                if self
                    .state
                    .stack_of_open_elements
                    .current_node_is(&NodeName::Tag(TagName::HTML))
                {
                    return self.step(NodeToProcess::ProcessNextNode);
                }

                /*
                 * > Otherwise, pop the current node from the stack of open elements.
                 */
                self.pop();

                /*
                 * > If the parser was not created as part of the HTML fragment parsing algorithm
                 * > (fragment case), and the current node is no longer a frameset element, then
                 * > switch the insertion mode to "after frameset".
                 */
                if self.context_node.is_none()
                    && !self
                        .state
                        .stack_of_open_elements
                        .current_node_is(&NodeName::Tag(TagName::FRAMESET))
                {
                    self.state.insertion_mode = InsertionMode::AFTER_FRAMESET;
                }

                true
            }

            /*
             * > A start tag whose tag name is "frame"
             *
             * > Insert an HTML element for the token. Immediately pop the
             * > current node off the stack of open elements.
             * >
             * > Acknowledge the token's self-closing flag, if it is set.
             */
            Op::TagPush(TagName::FRAME) => {
                self.insert_html_element(self.state.current_token.clone().unwrap());
                self.pop();
                true
            }

            /*
             * > A start tag whose tag name is "noframes"
             */
            Op::TagPush(TagName::NOFRAMES) => self.step_in_head(),

            /*
             * > Anything else
             */
            _ => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }
        }
    }

    /// Parses next element in the 'after frameset' insertion mode.
    ///
    /// This internal function performs the 'after frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterframeset
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_after_frameset(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * >   U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             * >
             * > Insert the character.
             *
             * This algorithm effectively strips non-whitespace characters from text and inserts
             * them under HTML. This is not supported at this time.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_body()
            }
            Op::Token(TokenType::Text) => {
                self.bail(UnsupportedException::NonWhitespaceCharsAfterFrameset)
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
             * > An end tag whose tag name is "html"
             */
            Op::TagPop(TagName::HTML) => {
                self.state.insertion_mode = InsertionMode::AFTER_AFTER_FRAMESET;
                /*
                 * The HTML element is not removed from the stack of open elements.
                 * Only internal state has changed, this does not qualify as a "step"
                 * in terms of advancing through the document to another token.
                 * Nothing has been pushed or popped.
                 * Proceed to parse the next item.
                 */
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "noframes"
             */
            Op::TagPush(TagName::NOFRAMES) => self.step_in_head(),

            /*
             * > Anything else
             * >   Parse error. Ignore the token.
             */
            _ => self.step(NodeToProcess::ProcessNextNode),
        }
    }

    /// Parses next element in the 'after after body' insertion mode.
    ///
    /// This internal function performs the 'after after body' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-body-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_after_after_body(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => self.bail(UnsupportedException::ContentOutsideOfHtml),

            /*
             * > A DOCTYPE token
             * > A start tag whose tag name is "html"
             *
             * > Process the token using the rules for the "in body" insertion mode.
             */
            Op::Token(TokenType::Doctype) | Op::TagPush(TagName::HTML) => self.step_in_body(),

            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * >   U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             * >
             * > Process the token using the rules for the "in body" insertion mode.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_body()
            }

            /*
             * > Anything else
             * > Parse error. Switch the insertion mode to "in body" and reprocess the token.
             */
            _ => {
                self.state.insertion_mode = InsertionMode::IN_BODY;
                self.step(NodeToProcess::ReprocessCurrentNode)
            }
        }
    }

    /// Parses next element in the 'after after frameset' insertion mode.
    ///
    /// This internal function performs the 'after after frameset' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-frameset-insertion-mode
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_after_after_frameset(&mut self) -> bool {
        match self.make_op() {
            /*
             * > A comment token
             */
            Op::Token(
                TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
            ) => self.bail(UnsupportedException::ContentOutsideOfHtml),

            /*
             * > A DOCTYPE token
             * > A start tag whose tag name is "html"
             *
             * > Process the token using the rules for the "in body" insertion mode.
             */
            Op::Token(TokenType::Doctype) | Op::TagPush(TagName::HTML) => self.step_in_body(),
            /*
             * > A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
             * >   U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
             * >
             * > Process the token using the rules for the "in body" insertion mode.
             *
             * This algorithm effectively strips non-whitespace characters from text and inserts
             * them under HTML. This is not supported at this time.
             */
            Op::Token(TokenType::Text)
                if self.tag_processor.text_node_classification
                    == TextNodeClassification::Whitespace =>
            {
                self.step_in_body()
            }
            Op::Token(TokenType::Text) => {
                self.bail(UnsupportedException::NonWhitespaceCharsAfterAfterFrameset)
            }

            /*
             * > A start tag whose tag name is "noframes"
             */
            Op::TagPush(TagName::NOFRAMES) => self.step_in_head(),

            /*
             * > Anything else
             * >   Parse error. Ignore the token.
             */
            _ => self.step(NodeToProcess::ProcessNextNode),
        }
    }

    /// Parses next element in the 'in foreign content' insertion mode.
    ///
    /// This internal function performs the 'in foreign content' insertion mode
    /// logic for the generalized WP_HTML_Processor::step() function.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inforeign
    /// @see WP_HTML_Processor::step
    ///
    /// @return bool Whether an element was found.
    fn step_in_foreign_content(&mut self) -> bool {
        let op = self.make_op();

        // Guards are at the pattern level, which is awkward to use.
        // Calculate this here to allow pattern matching the fond we're interested in.
        let is_font_with_special_attributes = matches!(op, Op::TagPush(TagName::FONT))
            && (self
                .get_attribute(b"color")
                .is_some_and(|attr| !matches!(attr, AttributeValue::BooleanFalse))
                || self
                    .get_attribute(b"face")
                    .is_some_and(|attr| !matches!(attr, AttributeValue::BooleanFalse))
                || self
                    .get_attribute(b"size")
                    .is_some_and(|attr| !matches!(attr, AttributeValue::BooleanFalse)));

        match (op, is_font_with_special_attributes) {
            (Op::Token(TokenType::Text), _) => {
                /*
                 * > A character token that is U+0000 NULL
                 *
                 * This is handled by `get_modifiable_text()`.
                 */

                /*
                 * Whitespace-only text does not affect the frameset-ok flag.
                 * It is probably inter-element whitespace, but it may also
                 * contain character references which decode only to whitespace.
                 */
                if self.tag_processor.text_node_classification == TextNodeClassification::Generic {
                    self.state.frameset_ok = false;
                }

                self.insert_foreign_element_from_current_token(false);
                true
            }

            /*
             * CDATA sections are alternate wrappers for text content and therefore
             * ought to follow the same rules as text nodes.
             */
            (Op::Token(TokenType::CdataSection), _) => {
                /*
                 * NULL bytes and whitespace do not change the frameset-ok flag.
                 */

                let current_token_span = self
                    .state
                    .current_token
                    .as_ref()
                    .and_then(|token| token.bookmark_name)
                    .and_then(|mark| self.tag_processor.internal_bookmarks.get(&mark))
                    .unwrap();

                let cdata_content_start = current_token_span.start + 9;
                let cdata_content_length = current_token_span.length - 12;
                if (strspn!(
                    &self.tag_processor.html_bytes,
                    b'\0' | b' ' | b'\t' | b'\n' | 0xc0 | b'\r',
                    cdata_content_start,
                    cdata_content_length
                ) != cdata_content_length)
                {
                    self.state.frameset_ok = false;
                }

                self.insert_foreign_element_from_current_token(false);
                true
            }

            /*
             * > A comment token
             */
            (
                Op::Token(
                    TokenType::Comment | TokenType::FunkyComment | TokenType::PresumptuousTag,
                ),
                _,
            ) => {
                self.insert_foreign_element_from_current_token(false);
                true
            }

            /*
             * > A DOCTYPE token
             */
            (Op::Token(TokenType::Doctype), _) => {
                // Parse error: ignore the token.
                self.step(NodeToProcess::ProcessNextNode)
            }

            /*
             * > A start tag whose tag name is "b", "big", "blockquote", "body", "br", "center",
             * > "code", "dd", "div", "dl", "dt", "em", "embed", "h1", "h2", "h3", "h4", "h5",
             * > "h6", "head", "hr", "i", "img", "li", "listing", "menu", "meta", "nobr", "ol",
             * > "p", "pre", "ruby", "s", "small", "span", "strong", "strike", "sub", "sup",
             * > "table", "tt", "u", "ul", "var"
             *
             * > A start tag whose name is "font", if the token has any attributes named "color", "face", or "size"
             *
             * > An end tag whose tag name is "br", "p"
             *
             * Closing BR tags are always reported by the Tag Processor as opening tags.
             */
            (
                Op::TagPush(
                    TagName::B
                    | TagName::BIG
                    | TagName::BLOCKQUOTE
                    | TagName::BODY
                    | TagName::BR
                    | TagName::CENTER
                    | TagName::CODE
                    | TagName::DD
                    | TagName::DIV
                    | TagName::DL
                    | TagName::DT
                    | TagName::EM
                    | TagName::EMBED
                    | TagName::H1
                    | TagName::H2
                    | TagName::H3
                    | TagName::H4
                    | TagName::H5
                    | TagName::H6
                    | TagName::HEAD
                    | TagName::HR
                    | TagName::I
                    | TagName::IMG
                    | TagName::LI
                    | TagName::LISTING
                    | TagName::MENU
                    | TagName::META
                    | TagName::NOBR
                    | TagName::OL
                    | TagName::P
                    | TagName::PRE
                    | TagName::RUBY
                    | TagName::S
                    | TagName::SMALL
                    | TagName::SPAN
                    | TagName::STRONG
                    | TagName::STRIKE
                    | TagName::SUB
                    | TagName::SUP
                    | TagName::TABLE
                    | TagName::TT
                    | TagName::U
                    | TagName::UL
                    | TagName::VAR,
                )
                | Op::TagPop(TagName::BR | TagName::P),
                _,
            )
            | (_, true) => {
                // @todo Indicate a parse error once it's possible.
                let pop_times = self.state.stack_of_open_elements.walk_up().position(
                    |HTMLToken {
                         namespace,
                         integration_node_type,
                         ..
                     }| {
                        &ParsingNamespace::Html == namespace
                            || matches!(
                                integration_node_type,
                                Some(IntegrationNodeType::HTML | IntegrationNodeType::MathML)
                            )
                    },
                );
                if let Some(pop_times) = pop_times {
                    for _ in 0..pop_times {
                        self.pop();
                    }
                }

                self.step_in_current_insertion_mode()
            }

            /*
             * > Any other start tag
             */
            (Op::TagPush(_), _) => {
                let has_self_closing_flag = self
                    .state
                    .current_token
                    .as_ref()
                    .unwrap()
                    .has_self_closing_flag;
                self.insert_foreign_element_from_current_token(false);

                /*
                 * > If the token has its self-closing flag set, then run
                 * > the appropriate steps from the following list:
                 * >
                 * >   ↪ the token's tag name is "script", and the new current node is in the SVG namespace
                 * >         Acknowledge the token's self-closing flag, and then act as
                 * >         described in the steps for a "script" end tag below.
                 * >
                 * >   ↪ Otherwise
                 * >         Pop the current node off the stack of open elements and
                 * >         acknowledge the token's self-closing flag.
                 *
                 * Since the rules for SCRIPT below indicate to pop the element off of the stack of
                 * open elements, which is the same for the Otherwise condition, there's no need to
                 * separate these checks. The difference comes when a parser operates with the scripting
                 * flag enabled, and executes the script, which this parser does not support.
                 */
                if has_self_closing_flag {
                    self.pop();
                }
                true
            }

            (Op::TagPop(TagName::SCRIPT), _)
                if self.state.current_token.as_ref().unwrap().namespace
                    == ParsingNamespace::Svg =>
            {
                self.pop();
                true
            }

            /*
             * > Any other end tag
             * >
             * >   Run these steps:
             * >     1. Initialize node to be the current node (the bottommost node of the stack).
             * >     2. If node's tag name, converted to ASCII lowercase, is not the same as the tag name of the token,
             * >        then this is a parse error.
             * >     3. Loop: If node is the topmost element in the stack of open elements, then return. (fragment case)
             * >     4. If node's tag name, converted to ASCII lowercase, is the same as the tag name of the token,
             * >        pop elements from the stack of open elements until node has been popped from the stack,
             * >        and then return.
             * >     5. Set node to the previous entry in the stack of open elements.
             * >     6. If node is not an element in the HTML namespace, return to the step labeled loop.
             * >     7. Otherwise, process the token according to the rules given in the section corresponding to
             * >        the current insertion mode in HTML content.
             */
            (Op::TagPop(tag_name), _) => {
                /*
                 * This section is just to produce a parse error that is not currently supported.
                 * Commented out.
                 *
                {
                    let initial_current_node = self
                        .state
                        .stack_of_open_elements
                        .current_node()
                        .expect("There must be a node on the stack.");
                    let current_node_tag_name = initial_current_node
                        .node_name
                        .tag()
                        .expect("There must be a tag name.");
                    if current_node_tag_name != &tag_name {
                        // @todo Indicate a parse error once it's possible.
                    }
                }
                */

                enum Continuation {
                    Unknown,
                    ProcessNextToken,
                    PopUntilTagName,
                    StepInCurrentInsertionMode,
                }

                let mut continuation = Continuation::Unknown;

                let mut first_iteration = true;
                for node in self.state.stack_of_open_elements.walk_up() {
                    // The spec describes some steps _after_ the updating the current node.
                    // Instead of manually looping, the steps are applied here after the
                    // first iteration.
                    if !first_iteration {
                        if node.namespace == ParsingNamespace::Html {
                            continuation = Continuation::StepInCurrentInsertionMode;
                            break;
                        }
                    } else {
                        first_iteration = false;
                    }

                    let node_tag_name = node.node_name.tag().expect("Must have a tag node.");
                    if node
                        == self
                            .state
                            .stack_of_open_elements
                            .at(1)
                            .expect("There must be a node on the stack.")
                    {
                        continuation = Continuation::ProcessNextToken;
                        break;
                    }

                    if node_tag_name == &tag_name {
                        continuation = Continuation::PopUntilTagName;
                        break;
                    }
                }

                match continuation {
                    Continuation::Unknown => {
                        unreachable!("Unknown foreign content end tag continuation.")
                    }
                    Continuation::PopUntilTagName => {
                        // See ::pop_until
                        while let Some(token) = self.pop() {
                            let token_node_name = token.node_name.tag();
                            if let Some(token_tag_name) = token_node_name {
                                if &tag_name == token_tag_name {
                                    return true;
                                }
                            }
                        }
                        unreachable!("Must have returned before reaching this point.");
                    }
                    Continuation::ProcessNextToken => self.step(NodeToProcess::ProcessNextNode),
                    Continuation::StepInCurrentInsertionMode => {
                        self.step_in_current_insertion_mode()
                    }
                }
            }

            (Op::Token(TokenType::Tag), _) => unreachable!("Tag token ops are never constructed"),
        }
    }

    /*
     * Internal helpers
     */

    /// Whether the processor paused because the input HTML document ended
    /// in the middle of a syntax element, such as in the middle of a tag.
    ///
    /// Example:
    ///
    ///     $processor = new WP_HTML_Tag_Processor( '<input type="text" value="Th' );
    ///     false      === $processor->get_next_tag();
    ///     true       === $processor->paused_at_incomplete_token();
    ///
    /// @return bool Whether the parse paused at the start of an incomplete token.
    pub fn paused_at_incomplete_token(&self) -> bool {
        self.tag_processor.paused_at_incomplete_token()
    }

    /// Creates a new bookmark for the currently-matched token and returns the generated name.
    ///
    /// @throws Exception When unable to allocate requested bookmark.
    ///
    /// @return string|false Name of created bookmark, or false if unable to create.
    fn bookmark_token(&mut self) -> Result<u32, HtmlProcessorError> {
        self.tag_processor
            .set_bookmark(BookmarkName::Internal(self.bookmark_counter + 1))
            .map(|_| {
                self.bookmark_counter += 1;
                self.bookmark_counter
            })
            .map_err(|_| HtmlProcessorError::ExceededMaxBookmarks)
    }

    /*
     * HTML semantic overrides for Tag Processor
     */

    /// Indicates the namespace of the current token, or "html" if there is none.
    ///
    /// @return string One of "html", "math", or "svg".
    pub fn get_namespace(&self) -> &ParsingNamespace {
        if let Some(current_element) = self.current_element.as_ref() {
            &current_element.token.namespace
        } else {
            self.tag_processor.get_namespace()
        }
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

        self.tag_processor.get_tag()
    }

    /// Returns the adjusted tag name for a given token, taking into
    /// account the current parsing context, whether HTML, SVG, or MathML.
    ///
    /// @return string|null Name of current tag name.
    pub fn get_qualified_tag_name(&self) -> Option<Box<[u8]>> {
        Some(self.get_tag()?.qualified_name(self.get_namespace()))
    }

    /// Returns the adjusted attribute name for a given attribute, taking into
    /// account the current parsing context, whether HTML, SVG, or MathML.
    ///
    /// @param string $attribute_name Which attribute to adjust.
    ///
    /// @return string|null
    pub fn get_qualified_attribute_name(&self, attribute_name: &[u8]) -> Option<Box<[u8]>> {
        if self.tag_processor.parser_state != ParserState::MatchedTag {
            return None;
        }
        Some(qualified_attribute_name(
            attribute_name,
            self.get_namespace(),
        ))
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

    /// Indicates what kind of matched token, if any.
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
    /// @return string|null What kind of token is matched, or null.
    pub fn get_token_type(&self) -> Option<&TokenType> {
        if self.is_virtual() {
            /*
             * This logic comes from the Tag Processor.
             *
             * @todo It would be ideal not to repeat this here, but it's not clearly
             *       better to allow passing a token name to `get_token_type()`.
             */
            Some(
                match &self.current_element.as_ref().unwrap().token.node_name {
                    NodeName::Tag(_) => &TokenType::Tag,
                    NodeName::Token(token_type) => token_type,
                },
            )
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
    /// @param string $name Name of attribute whose value is requested.
    /// @return string|true|null Value of attribute or `null` if not available. Boolean attributes return `true`.
    pub fn get_attribute(&self, name: &[u8]) -> Option<AttributeValue> {
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
    /// - HTML 5 spec
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
    /// @see https://html.spec.whatwg.org/multipage/syntax.html#attributes-2:ascii-case-insensitive
    ///
    /// @param string $prefix Prefix of requested attribute names.
    /// @return array|null List of attribute names, or `null` when no tag opener is matched.
    pub fn get_attribute_names_with_prefix(&self, prefix: &[u8]) -> Option<Vec<Box<[u8]>>> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.get_attribute_names_with_prefix(prefix)
        }
    }

    /// Adds a new class name to the currently matched tag.
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
    pub fn class_list(&self) {
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
    /// @return string
    pub fn get_modifiable_text(&self) -> Box<[u8]> {
        if self.is_virtual() {
            Box::new([])
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
    /// @return string|null
    pub fn get_comment_type(&self) -> Option<&CommentType> {
        if self.is_virtual() {
            None
        } else {
            self.tag_processor.get_comment_type()
        }
    }

    /// Returns the text of a matched comment or null if not on a comment type node.
    ///
    /// This method returns the entire text content of a comment node as it
    /// would appear in the browser.
    ///
    /// This differs from {@see ::get_modifiable_text()} in that certain comment
    /// types in the HTML API cannot allow their entire comment text content to
    /// be modified. Namely, "bogus comments" of the form `<?not allowed in html>`
    /// will create a comment whose text content starts with `?`. Note that if
    /// that character were modified, it would be possible to change the node
    /// type.
    ///
    /// @return string|null The comment text as it would appear in the browser or null
    ///                     if not on a comment type node.
    pub fn get_full_comment_text(&self) -> Option<Box<[u8]>> {
        self.tag_processor.get_full_comment_text()
    }

    /// Removes a bookmark that is no longer needed.
    ///
    /// Releasing a bookmark frees up the small
    /// performance overhead it requires.
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
    /// @param string $bookmark_name Identifies this particular bookmark.
    /// @return bool Whether the bookmark was successfully created.
    pub fn set_bookmark(&mut self, bookmark_name: &str) -> Result<(), ()> {
        let bookmark_name = format!("_{}", bookmark_name);
        self.tag_processor.set_bookmark(bookmark_name.as_str())
    }

    /// Checks whether a bookmark with the given name exists.
    ///
    /// @param string $bookmark_name Name to identify a bookmark that potentially exists.
    /// @return bool Whether that bookmark exists.
    pub fn has_bookmark(&self, bookmark_name: &str) -> bool {
        let bookmark_name = format!("_{}", bookmark_name);
        self.tag_processor.has_bookmark(&bookmark_name)
    }

    /*
     *
     * HTML Parsing Algorithms
     *
     */

    /// Closes a P element.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#close-a-p-element
    fn close_a_p_element(&mut self) {
        self.generate_implied_end_tags(Some(&TagName::P));
        self.pop_until(&TagName::P);
    }

    /// Closes elements that have implied end tags.
    ///
    /// > while the current node is a dd element, a dt element, an li element,
    /// > an optgroup element, an option element, a p element, an rb element,
    /// > an rp element, an rt element, or an rtc element,
    /// > the UA must pop the current node off the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/#generate-implied-end-tags
    ///
    /// @param string|null $except_for_this_element Perform as if this element doesn't exist in the stack of open elements.
    fn generate_implied_end_tags(&mut self, except_for_this_element: Option<&TagName>) {
        while let Some(token) = self.state.stack_of_open_elements.current_node() {
            if token.namespace != ParsingNamespace::Html {
                return;
            }

            match &token.node_name {
                NodeName::Tag(
                    current_tag @ (TagName::DD
                    | TagName::DT
                    | TagName::LI
                    | TagName::OPTGROUP
                    | TagName::OPTION
                    | TagName::P
                    | TagName::RB
                    | TagName::RP
                    | TagName::RT
                    | TagName::RTC),
                ) if Some(current_tag) != except_for_this_element => {
                    self.pop();
                }
                NodeName::Tag(_) => return,
                NodeName::Token(_) => return,
            }
        }
    }

    /// Closes elements that have implied end tags, thoroughly.
    ///
    /// See the HTML specification for an explanation why this is
    /// different from generating end tags in the normal sense.
    ///
    /// @see WP_HTML_Processor::generate_implied_end_tags
    /// @see https://html.spec.whatwg.org/#generate-implied-end-tags
    fn generate_implied_end_tags_thoroughly(&mut self) {
        while let Some(token) = self.state.stack_of_open_elements.current_node() {
            if token.namespace != ParsingNamespace::Html {
                return;
            }

            match &token.node_name {
                NodeName::Tag(
                    TagName::CAPTION
                    | TagName::COLGROUP
                    | TagName::DD
                    | TagName::DT
                    | TagName::LI
                    | TagName::OPTGROUP
                    | TagName::OPTION
                    | TagName::P
                    | TagName::RB
                    | TagName::RP
                    | TagName::RT
                    | TagName::RTC
                    | TagName::TBODY
                    | TagName::TD
                    | TagName::TFOOT
                    | TagName::TH
                    | TagName::THEAD
                    | TagName::TR,
                ) => {
                    self.pop();
                }
                NodeName::Tag(_) => return,
                NodeName::Token(_) => return,
            }
        }
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
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#reconstruct-the-active-formatting-elements
    ///
    /// @return bool Whether any formatting elements needed to be reconstructed.
    fn reconstruct_active_formatting_elements(&mut self) -> bool {
        /*
         * > If there are no entries in the list of active formatting elements, then there is nothing
         * > to reconstruct; stop this algorithm.
         */
        if self.state.active_formatting_elements.count() == 0 {
            return false;
        }

        let last_entry = self
            .state
            .active_formatting_elements
            .current_node()
            .unwrap();

        let last_entry = match last_entry {
            ActiveFormattingElement::Token(token) => token,
            /*
             * > If the last (most recently added) entry in the list of active formatting elements is a marker;
             * > stop this algorithm.
             */
            ActiveFormattingElement::Marker => {
                return false;
            }
        };

        /*
         * > If the last (most recently added) entry in the list of active formatting elements is an
         * > element that is in the stack of open elements, then there is nothing to reconstruct;
         * > stop this algorithm.
         */
        if self.state.stack_of_open_elements.contains_node(last_entry) {
            return false;
        }

        self.bail(UnsupportedException::ActiveFormattingElementsWhenAdvancingAndRewindingIsRequired)
    }

    /// Runs the reset the insertion mode appropriately algorithm.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#reset-the-insertion-mode-appropriately
    fn reset_insertion_mode_appropriately(&mut self) {
        // Set the first node.
        let first_node = self
            .state
            .stack_of_open_elements
            .walk_down()
            .next()
            .expect("Cannot reset insertion mode with an empty stack of open elements");

        /*
         * > 1. Let _last_ be false.
         */
        let mut last = false;
        for mut node in self.state.stack_of_open_elements.walk_up() {
            /*
             * > 2. Let _node_ be the last node in the stack of open elements.
             * > 3. _Loop_: If _node_ is the first node in the stack of open elements, then set _last_
             * >            to true, and, if the parser was created as part of the HTML fragment parsing
             * >            algorithm (fragment case), set node to the context element passed to
             * >            that algorithm.
             * > …
             */
            if node == first_node {
                last = true;
                node = if let Some(context_node) = &self.context_node {
                    context_node
                } else {
                    node
                };
            }

            // All of the following rules are for matching HTML elements.
            if node.namespace != ParsingNamespace::Html {
                continue;
            }

            // This function is not interested in tokens, only tags.
            let node_tag = if let NodeName::Tag(tag_name) = &node.node_name {
                tag_name
            } else {
                continue;
            };

            match node_tag {
                /*
                 * > 4. If node is a `select` element, run these substeps:
                 * >   1. If _last_ is true, jump to the step below labeled done.
                 * >   2. Let _ancestor_ be _node_.
                 * >   3. _Loop_: If _ancestor_ is the first node in the stack of open elements,
                 * >      jump to the step below labeled done.
                 * >   4. Let ancestor be the node before ancestor in the stack of open elements.
                 * >   …
                 * >   7. Jump back to the step labeled _loop_.
                 * >   8. _Done_: Switch the insertion mode to "in select" and return.
                 */
                TagName::SELECT => {
                    if !last {
                        for ancestor in self
                            .state
                            .stack_of_open_elements
                            .walk_up()
                            .skip_while(|&ancestor| ancestor != node)
                            .skip(1)
                        {
                            if node == first_node {
                                break;
                            }

                            if ancestor.namespace != ParsingNamespace::Html {
                                continue;
                            }

                            // This function is not interested in tokens, only tags.
                            let ancestor_tag = if let Some(tag_name) = ancestor.node_name.tag() {
                                tag_name
                            } else {
                                continue;
                            };

                            match ancestor_tag {
                                /*
                                 * > 5. If _ancestor_ is a `template` node, jump to the step below
                                 * >    labeled _done_.
                                 */
                                TagName::TEMPLATE => {
                                    break;
                                }

                                /*
                                 * > 6. If _ancestor_ is a `table` node, switch the insertion mode to
                                 * >    "in select in table" and return.
                                 */
                                TagName::TABLE => {
                                    self.state.insertion_mode = InsertionMode::IN_SELECT_IN_TABLE;
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                    self.state.insertion_mode = InsertionMode::IN_SELECT;
                    return;
                }

                /*
                 * > 5. If _node_ is a `td` or `th` element and _last_ is false, then switch the
                 * >    insertion mode to "in cell" and return.
                 */
                TagName::TD | TagName::TH if !last => {
                    self.state.insertion_mode = InsertionMode::IN_CELL;
                    return;
                }

                /*
                 * > 6. If _node_ is a `tr` element, then switch the insertion mode to "in row"
                 * >    and return.
                 */
                TagName::TR => {
                    self.state.insertion_mode = InsertionMode::IN_ROW;
                    return;
                }

                /*
                 * > 7. If _node_ is a `tbody`, `thead`, or `tfoot` element, then switch the insertion mode to "in table body" and return.
                 */
                TagName::TBODY | TagName::THEAD | TagName::TFOOT => {
                    self.state.insertion_mode = InsertionMode::IN_TABLE_BODY;
                    return;
                }

                /*
                 * > 8. If _node_ is a `caption` element, then switch the insertion mode to "in caption" and return.
                 */
                TagName::CAPTION => {
                    self.state.insertion_mode = InsertionMode::IN_CAPTION;
                    return;
                }

                /*
                 * > 9. If _node_ is a `colgroup` element, then switch the insertion mode to "in column group" and return.
                 */
                TagName::COLGROUP => {
                    self.state.insertion_mode = InsertionMode::IN_COLUMN_GROUP;
                    return;
                }

                /*
                 * > 10. If _node_ is a `table` element, then switch the insertion mode to "in table" and return.
                 */
                TagName::TABLE => {
                    self.state.insertion_mode = InsertionMode::IN_TABLE;
                    return;
                }

                /*
                 * > 11. If _node_ is a `template` element, then switch the insertion mode to the
                 * >     current template insertion mode and return.
                 */
                TagName::TEMPLATE => {
                    self.state.insertion_mode = self.state.stack_of_template_insertion_modes.last().expect("There must be a template insertion mode to reset the insertion mode appropriately.").clone();
                    return;
                }

                /*
                 * > 12. If _node_ is a `head` element and _last_ is false, then switch the
                 * >     insertion mode to "in head" and return.
                 */
                TagName::HEAD if !last => {
                    self.state.insertion_mode = InsertionMode::IN_HEAD;
                    return;
                }

                /*
                 * > 13. If _node_ is a `body` element, then switch the insertion mode to "in body"
                 * >     and return.
                 */
                TagName::BODY => {
                    self.state.insertion_mode = InsertionMode::IN_BODY;
                    return;
                }

                /*
                 * > 14. If _node_ is a `frameset` element, then switch the insertion mode to
                 * >     "in frameset" and return. (fragment case)
                 */
                TagName::FRAMESET => {
                    self.state.insertion_mode = InsertionMode::IN_FRAMESET;
                    return;
                }

                /*
                 * > 15. If _node_ is an `html` element, run these substeps:
                 * >     1. If the head element pointer is null, switch the insertion mode to
                 * >        "before head" and return. (fragment case)
                 * >     2. Otherwise, the head element pointer is not null, switch the insertion
                 * >        mode to "after head" and return.
                 */
                TagName::HTML => {
                    self.state.insertion_mode = match self.state.head_element {
                        None => InsertionMode::BEFORE_HEAD,
                        Some(_) => InsertionMode::AFTER_HEAD,
                    };
                    return;
                }

                /*
                 * > 16. If _last_ is true, then switch the insertion mode to "in body"
                 * >     and return. (fragment case)
                 *
                 * This is only reachable if `$last` is true, as per the fragment parsing case.
                 */
                _ if last => {
                    self.state.insertion_mode = InsertionMode::IN_BODY;
                }

                _ => {}
            }
        }

        /*
         * > 16. If _last_ is true, then switch the insertion mode to "in body"
         * >     and return. (fragment case)
         *
         * This is only reachable if `$last` is true, as per the fragment parsing case.
         */
        self.state.insertion_mode = InsertionMode::IN_BODY;
    }

    /// Runs the adoption agency algorithm.
    ///
    /// @throws WP_HTML_Unsupported_Exception When encountering unsupported HTML input.
    ///
    /// @see https://html.spec.whatwg.org/#adoption-agency-algorithm
    fn run_adoption_agency_algorithm(&mut self) {
        let mut budget: u16 = 1_000;
        let subject = &self.get_tag().unwrap();
        let current_node = self.state.stack_of_open_elements.current_node();

        // > If the current node is an HTML element whose tag name is subject
        if let Some(
            token @ HTMLToken {
                node_name: NodeName::Tag(current_node_tag_name),
                ..
            },
        ) = current_node
        {
            if subject == current_node_tag_name
            // > the current node is not in the list of active formatting elements
                    && !self
                        .state
                        .active_formatting_elements
                        .contains_node(token)
            {
                self.pop();
                return;
            }
        }

        let mut outer_loop_counter: u8 = 0;
        while budget > 0 {
            budget -= 1;
            if outer_loop_counter >= 8 {
                return;
            }
            outer_loop_counter += 1;

            /*
             * > Let formatting element be the last element in the list of active formatting elements that:
             * >   - is between the end of the list and the last marker in the list,
             * >     if any, or the start of the list otherwise,
             * >   - and has the tag name subject.
             *
             * // @todo this looks like a find?
             */
            let mut formatting_element = None;
            for item in self.state.active_formatting_elements.walk_up() {
                match item {
                    ActiveFormattingElement::Marker => break,
                    ActiveFormattingElement::Token(token) => {
                        if let NodeName::Tag(tag_name) = &token.node_name {
                            if subject == tag_name {
                                formatting_element = Some(token.clone());
                                break;
                            }
                        }
                    }
                }
            }

            // > If there is no such element, then return and instead act as described in the "any other end tag" entry above.
            let formatting_element = match formatting_element {
                Some(element) => element,
                None => {
                    self.bail(UnsupportedException::AdoptionAgencyWhenAnyOtherEndTagIsRequired);
                    return;
                }
            };

            // > If formatting element is not in the stack of open elements, then this is a parse error; remove the element from the list, and return.
            if !self
                .state
                .stack_of_open_elements
                .contains_node(&formatting_element)
            {
                self.state
                    .active_formatting_elements
                    .remove_node(&formatting_element);
                return;
            }

            // > If formatting element is in the stack of open elements, but the element is not in scope, then this is a parse error; return.
            if !self
                .state
                .stack_of_open_elements
                .has_element_in_scope(subject)
            {
                return;
            }

            /*
             * > Let furthest block be the topmost node in the stack of open elements that is lower in the stack
             * > than formatting element, and is an element in the special category. There might not be one.
             */
            let mut is_above_formatting_element = true;
            let mut furthest_block = None;
            for item in self.state.stack_of_open_elements.walk_down() {
                if is_above_formatting_element
                    && formatting_element.bookmark_name != item.bookmark_name
                {
                    continue;
                }

                if is_above_formatting_element {
                    is_above_formatting_element = false;
                    continue;
                }

                if let NodeName::Tag(tag_name) = &item.node_name {
                    if Self::is_special(tag_name) {
                        furthest_block = Some(item.clone());
                        break;
                    }
                }
            }

            /*
             * > If there is no furthest block, then the UA must first pop all the nodes from the bottom of the
             * > stack of open elements, from the current node up to and including formatting element, then
             * > remove formatting element from the list of active formatting elements, and finally return.
             */
            if furthest_block.is_none() {
                while let Some(x) = self.pop() {
                    if x == formatting_element {
                        break;
                    }
                }

                self.state
                    .active_formatting_elements
                    .remove_node(&formatting_element);
                return;
            }

            self.bail(UnsupportedException::AdoptionAgencyCannotExtractCommonAncestor);
            return;
        }

        self.bail(UnsupportedException::AdoptionAgencyWhenLoopingRequired);
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
    fn close_cell(&mut self) {
        self.generate_implied_end_tags(None);

        // @todo Parse error if the current node is a "td" or "th" element.
        while let Some(HTMLToken {
            node_name: popped_token_node_name,
            ..
        }) = self.pop()
        {
            if matches!(
                popped_token_node_name,
                NodeName::Tag(TagName::TD | TagName::TH)
            ) {
                break;
            }
        }

        self.state
            .active_formatting_elements
            .clear_up_to_last_marker();
        self.state.insertion_mode = InsertionMode::IN_ROW;
    }

    /// Inserts an HTML element on the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/#insert-a-foreign-element
    ///
    /// @param WP_HTML_Token $token Name of bookmark pointing to element in original input HTML.
    fn insert_html_element(&mut self, token: HTMLToken) {
        self.push(token);
    }

    /// Inserts a foreign element on to the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/#insert-a-foreign-element
    ///
    /// @param WP_HTML_Token $token                     Insert this token. The token's namespace and
    ///                                                 insertion point will be updated correctly.
    /// @param bool          $only_add_to_element_stack Whether to skip the "insert an element at the adjusted
    ///                                                 insertion location" algorithm when adding this element.
    fn insert_foreign_element_from_current_token(&mut self, only_add_to_element_stack: bool) {
        let adjusted_namespace = self
            .get_adjusted_current_node()
            .map_or(ParsingNamespace::Html, |tok| tok.namespace.clone());

        if let Some(token) = self.state.current_token.as_mut() {
            token.namespace = adjusted_namespace;
        }

        if self.is_mathml_integration_point() {
            if let Some(token) = self.state.current_token.as_mut() {
                token.integration_node_type = Some(IntegrationNodeType::MathML);
            }
        } else if self.is_html_integration_point() {
            if let Some(token) = self.state.current_token.as_mut() {
                token.integration_node_type = Some(IntegrationNodeType::HTML);
            }
        }

        if !only_add_to_element_stack {
            /*
             * @todo Implement the "appropriate place for inserting a node" and the
             *       "insert an element at the adjusted insertion location" algorithms.
             *
             * These algorithms mostly impacts DOM tree construction and not the HTML API.
             * Here, there's no DOM node onto which the element will be appended, so the
             * parser will skip this step.
             *
             * @see https://html.spec.whatwg.org/#insert-an-element-at-the-adjusted-insertion-location
             */
        }

        self.insert_html_element(self.state.current_token.as_ref().unwrap().clone());
    }

    /// Inserts a virtual element on the stack of open elements.
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
        let current_token_start = self
            .state
            .current_token
            .as_ref()
            .and_then(|token| token.bookmark_name)
            .and_then(|mark| self.tag_processor.internal_bookmarks.get(&mark))
            .map(|span| span.start)
            .unwrap();

        let name = self.bookmark_token().unwrap();
        self.tag_processor
            .internal_bookmarks
            .insert(name, HtmlSpan::new(current_token_start, 0));
        let token = HTMLToken::new(Some(name), token_name.into(), false);
        self.insert_html_element(token.clone());
        token
    }

    /*
     *
     * HTML Specification Helpers
     *
     */

    /// Indicates if the current token is a MathML integration point.
    ///
    /// @see https://html.spec.whatwg.org/#mathml-text-integration-point
    ///
    /// @return bool Whether the current token is a MathML integration point.
    fn is_mathml_integration_point(&self) -> bool {
        let current_token = match &self.state.current_token {
            Some(token) => token,
            None => return false,
        };

        if current_token.namespace != ParsingNamespace::MathML {
            return false;
        }

        let tag_name = match &current_token.node_name {
            NodeName::Tag(tag_name) => tag_name,
            NodeName::Token(_) => return false,
        };

        matches!(
            tag_name,
            TagName::MI | TagName::MO | TagName::MN | TagName::MS | TagName::MTEXT
        )
    }

    /// Indicates if the current token is an HTML integration point.
    ///
    /// Note that this method must be an instance method with access
    /// to the current token, since it needs to examine the attributes
    /// of the currently-matched tag, if it's in the MathML namespace.
    /// Otherwise it would be required to scan the HTML and ensure that
    /// no other accounting is overlooked.
    ///
    /// @see https://html.spec.whatwg.org/#html-integration-point
    ///
    /// @return bool Whether the current token is an HTML integration point.
    fn is_html_integration_point(&self) -> bool {
        let current_token = match &self.state.current_token {
            Some(token) => token,
            None => return false,
        };

        let tag_name = match &current_token.node_name {
            NodeName::Tag(tag_name) => tag_name,
            NodeName::Token(_) => return false,
        };

        match current_token.namespace {
            ParsingNamespace::Html => false,
            ParsingNamespace::MathML => {
                tag_name == &TagName::ANNOTATION_XML
                    && self
                        .get_attribute(b"encoding")
                        .is_some_and(|encoding| match encoding {
                            AttributeValue::String(encoding) => {
                                encoding.eq_ignore_ascii_case(b"application/xhtml+xml")
                                    || encoding.eq_ignore_ascii_case(b"text/html")
                            }
                            _ => false,
                        })
            }
            ParsingNamespace::Svg => {
                matches!(
                    tag_name,
                    TagName::DESC | TagName::FOREIGNOBJECT | TagName::TITLE
                )
            }
        }
    }

    /// Returns whether an element of a given name is in the HTML special category.
    ///
    /// @see https://html.spec.whatwg.org/#special
    ///
    /// @param WP_HTML_Token|string $tag_name Node to check, or only its name if in the HTML namespace.
    /// @return bool Whether the element of the given name is in the special category.
    pub fn is_special(tag_name: &TagName) -> bool {
        matches!(
            tag_name,
            TagName::ADDRESS
                | TagName::APPLET
                | TagName::AREA
                | TagName::ARTICLE
                | TagName::ASIDE
                | TagName::BASE
                | TagName::BASEFONT
                | TagName::BGSOUND
                | TagName::BLOCKQUOTE
                | TagName::BODY
                | TagName::BR
                | TagName::BUTTON
                | TagName::CAPTION
                | TagName::CENTER
                | TagName::COL
                | TagName::COLGROUP
                | TagName::DD
                | TagName::DETAILS
                | TagName::DIR
                | TagName::DIV
                | TagName::DL
                | TagName::DT
                | TagName::EMBED
                | TagName::FIELDSET
                | TagName::FIGCAPTION
                | TagName::FIGURE
                | TagName::FOOTER
                | TagName::FORM
                | TagName::FRAME
                | TagName::FRAMESET
                | TagName::H1
                | TagName::H2
                | TagName::H3
                | TagName::H4
                | TagName::H5
                | TagName::H6
                | TagName::HEAD
                | TagName::HEADER
                | TagName::HGROUP
                | TagName::HR
                | TagName::HTML
                | TagName::IFRAME
                | TagName::IMG
                | TagName::INPUT
                | TagName::KEYGEN
                | TagName::LI
                | TagName::LINK
                | TagName::LISTING
                | TagName::MAIN
                | TagName::MARQUEE
                | TagName::MENU
                | TagName::META
                | TagName::NAV
                | TagName::NOEMBED
                | TagName::NOFRAMES
                | TagName::NOSCRIPT
                | TagName::OBJECT
                | TagName::OL
                | TagName::P
                | TagName::PARAM
                | TagName::PLAINTEXT
                | TagName::PRE
                | TagName::SCRIPT
                | TagName::SEARCH
                | TagName::SECTION
                | TagName::SELECT
                | TagName::SOURCE
                | TagName::STYLE
                | TagName::SUMMARY
                | TagName::TABLE
                | TagName::TBODY
                | TagName::TD
                | TagName::TEMPLATE
                | TagName::TEXTAREA
                | TagName::TFOOT
                | TagName::TH
                | TagName::THEAD
                | TagName::TITLE
                | TagName::TR
                | TagName::TRACK
                | TagName::UL
                | TagName::WBR
                | TagName::XMP
                | TagName::MI
                | TagName::MO
                | TagName::MN
                | TagName::MS
                | TagName::MTEXT
                | TagName::ANNOTATION_XML
                | TagName::DESC
                | TagName::FOREIGNOBJECT
        )
    }

    /// Returns whether a given element is an HTML Void Element
    ///
    /// > area, base, br, col, embed, hr, img, input, link, meta, source, track, wbr
    ///
    /// @see https://html.spec.whatwg.org/#void-elements
    ///
    /// @param string $tag_name Name of HTML tag to check.
    /// @return bool Whether the given tag is an HTML Void Element.
    pub fn is_void(tag_name: &TagName) -> bool {
        matches!(
            tag_name,
            TagName::AREA
                | TagName::BASE
                | TagName::BASEFONT // Obsolete but still treated as void.
                | TagName::BGSOUND // Obsolete but still treated as void.
                | TagName::BR
                | TagName::COL
                | TagName::EMBED
                | TagName::FRAME
                | TagName::HR
                | TagName::IMG
                | TagName::INPUT
                | TagName::KEYGEN // Obsolete but still treated as void.
                | TagName::LINK
                | TagName::META
                | TagName::PARAM // Obsolete but still treated as void.
                | TagName::SOURCE
                | TagName::TRACK
                | TagName::WBR
        )
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
    /// @param string $label A string which may specify a known encoding.
    /// @return string|null Known encoding if matched, otherwise null.
    ///
    /// @todo What do wo with this _protected_ function?
    fn get_encoding(label: &str) -> Option<Rc<str>> {
        todo!()
    }

    fn make_op(&self) -> Op {
        match self.get_token_type() {
            Some(TokenType::Tag) if self.is_tag_closer() => Op::TagPop(self.get_tag().unwrap()),
            Some(TokenType::Tag) => Op::TagPush(self.get_tag().unwrap()),
            Some(
                token @ (TokenType::CdataSection
                | TokenType::Comment
                | TokenType::Doctype
                | TokenType::FunkyComment
                | TokenType::PresumptuousTag
                | TokenType::Text),
            ) => Op::Token(token.clone()),
            None => unreachable!("Op should never be made when no token is available."),
        }
    }

    fn push(&mut self, token: HTMLToken) {
        self.state.stack_of_open_elements._push(token.clone());

        let is_virtual = self.state.current_token.is_none() || self.is_tag_closer();
        let same_node = self
            .state
            .current_token
            .as_ref()
            .is_some_and(|current| current.node_name == token.node_name);
        let provenance = if !same_node || is_virtual {
            StackProvenance::Virtual
        } else {
            StackProvenance::Real
        };
        self.element_queue.push_back(HTMLStackEvent {
            token: token.clone(),
            operation: StackOperation::Push,
            provenance,
        });

        self.tag_processor
            .change_parsing_namespace(if token.integration_node_type.is_some() {
                ParsingNamespace::Html
            } else {
                token.namespace
            });
    }

    fn pop(&mut self) -> Option<HTMLToken> {
        let token = self.state.stack_of_open_elements._pop()?;
        self.after_pop(&token);
        Some(token)
    }

    fn after_pop(&mut self, token: &HTMLToken) {
        if let Some(bookmark_name) = token.bookmark_name.as_ref() {
            let _ = self.tag_processor.internal_bookmarks.remove(bookmark_name);
        }

        let is_virtual = self.state.current_token.is_none() || !self.is_tag_closer();
        let same_node = self
            .state
            .current_token
            .as_ref()
            .is_some_and(|current| current.node_name == token.node_name);
        let provenance = if !same_node || is_virtual {
            StackProvenance::Virtual
        } else {
            StackProvenance::Real
        };
        self.element_queue.push_back(HTMLStackEvent {
            token: token.clone(),
            operation: StackOperation::Pop,
            provenance,
        });

        if let Some(adjusted_current_node) = self.get_adjusted_current_node() {
            self.tag_processor.change_parsing_namespace(
                if adjusted_current_node.integration_node_type.is_some() {
                    ParsingNamespace::Html
                } else {
                    adjusted_current_node.namespace.clone()
                },
            );
        } else {
            self.tag_processor
                .change_parsing_namespace(ParsingNamespace::Html);
        };
    }

    /// Pops nodes off of the stack of open elements until an HTML tag with the given name has been popped.
    ///
    /// In the PHP implementation, this method exists on the stack of open elements class.
    ///
    /// @param string $html_tag_name Name of tag that needs to be popped off of the stack of open elements.
    /// @return bool Whether a tag of the given name was found and popped off of the stack of open elements.
    fn pop_until(&mut self, html_tag_name: &TagName) -> bool {
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

    /// Pop until any H1-H6 element has been popped off of the stack of open elements.
    ///
    /// !!! This function does not exist in the PHP implementation !!!
    ///
    /// Most pop_until usage is for a single element. The H1-H6 elements are an
    /// exception and this additional method prevents needing to implement checks for multiple
    /// elements.
    ///
    /// The
    fn pop_until_any_h1_to_h6(&mut self) -> bool {
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

    /// Removes a specific node from the stack of open elements.
    ///
    /// @param WP_HTML_Token $token The node to remove from the stack of open elements.
    /// @return bool Whether the node was found and removed from the stack of open elements.
    fn remove_node_from_stack_of_open_elements(&mut self, token: &HTMLToken) -> bool {
        if let Some(idx) = self
            .state
            .stack_of_open_elements
            .stack
            .iter()
            .rev()
            .position(|item| item == token)
        {
            let idx = self.state.stack_of_open_elements.stack.len() - 1 - idx;
            let token = self.state.stack_of_open_elements.stack.remove(idx);
            self.after_pop(&token);
            true
        } else {
            false
        }
    }

    /// Clear the stack back to a table context.
    ///
    /// > When the steps above require the UA to clear the stack back to a table context, it means
    /// > that the UA must, while the current node is not a table, template, or html element, pop
    /// > elements from the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#clear-the-stack-back-to-a-table-context
    fn clear_to_table_context(&mut self) {
        while let Some(HTMLToken { node_name, .. }) =
            self.state.stack_of_open_elements.current_node()
        {
            if matches!(
                node_name,
                NodeName::Tag(TagName::TABLE | TagName::TEMPLATE | TagName::HTML)
            ) {
                break;
            }
            self.pop();
        }
    }

    /// Clear the stack back to a table body context.
    ///
    /// > When the steps above require the UA to clear the stack back to a table body context, it
    /// > means that the UA must, while the current node is not a tbody, tfoot, thead, template, or
    /// > html element, pop elements from the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#clear-the-stack-back-to-a-table-body-context
    fn clear_to_table_body_context(&mut self) {
        while let Some(HTMLToken { node_name, .. }) =
            self.state.stack_of_open_elements.current_node()
        {
            if matches!(
                node_name,
                NodeName::Tag(
                    TagName::TBODY
                        | TagName::TFOOT
                        | TagName::THEAD
                        | TagName::TEMPLATE
                        | TagName::HTML
                )
            ) {
                break;
            }
            self.pop();
        }
    }

    /// Clear the stack back to a table row context.
    ///
    /// > When the steps above require the UA to clear the stack back to a table row context, it
    /// > means that the UA must, while the current node is not a tr, template, or html element, pop
    /// > elements from the stack of open elements.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#clear-the-stack-back-to-a-table-row-context
    fn clear_to_table_row_context(&mut self) {
        while let Some(HTMLToken { node_name, .. }) =
            self.state.stack_of_open_elements.current_node()
        {
            if matches!(
                node_name,
                NodeName::Tag(TagName::TR | TagName::TEMPLATE | TagName::HTML)
            ) {
                break;
            }
            self.pop();
        }
    }
}

#[derive(Debug, PartialEq)]
enum Op {
    TagPush(TagName),
    TagPop(TagName),
    Token(TokenType),
}
