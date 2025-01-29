#![allow(dead_code)]

use std::ops::Deref;

use ext_php_rs::props::Prop;

pub struct HtmlProcessor {
    attributes: (),
    bytes_already_parsed: usize,
    comment_type: Option<CommentType>,
    duplicate_attributes: Option<Vec<HtmlSpan>>,
    html: Box<str>,
    is_closing_tag: Option<bool>,
    lexical_updates: Vec<HtmlTextReplacement>,
    parser_state: ProcessorState,
    parsing_namespace: ParsingNamespace,
    skip_newline_at: Option<usize>,
    tag_name_length: Option<usize>,
    tag_name_starts_at: Option<usize>,
    text_length: Option<usize>,
    text_node_classification: TextNodeClassification,
    text_starts_at: Option<usize>,
    token_length: Option<usize>,
    token_starts_at: Option<usize>,
}

#[derive(Default, PartialEq)]
enum ParsingNamespace {
    #[default]
    Html,
    Svg,
}

struct HtmlTextReplacement {
    start: usize,
    length: usize,
    text: Box<str>,
}

#[derive(Clone)]
struct HtmlSpan {
    start: usize,
    length: usize,
}

impl HtmlTextReplacement {
    pub fn new(start: usize, length: usize, text: Box<str>) -> Self {
        Self {
            start,
            length,
            text,
        }
    }
}

impl HtmlProcessor {
    pub fn create_fragment(html: &str) -> Self {
        Self {
            html: html.to_string().into_boxed_str(),
            ..Default::default()
        }
    }

    pub fn next_token(&mut self) -> bool {
        self.base_class_next_token()
    }

    fn base_class_next_token(&mut self) -> bool {
        let was_at = self.bytes_already_parsed;
        self.after_tag();

        if self.parser_state == ProcessorState::Complete
            || self.parser_state == ProcessorState::IncompleteInput
        {
            return false;
        }

        /*
         * The next step in the parsing loop determines the parsing state;
         * clear it so that state doesn't linger from the previous step.
         */
        self.parser_state = ProcessorState::Ready;

        if self.bytes_already_parsed >= self.html.len() {
            self.parser_state = ProcessorState::Complete;
            return false;
        }

        // Find the next tag if it exists.
        if false == self.parse_next_tag() {
            if self.parser_state == ProcessorState::IncompleteInput {
                self.bytes_already_parsed = was_at;
            }

            return false;
        }

        /*
         * For legacy reasons the rest of this function handles tags and their
         * attributes. If the processor has reached the end of the document
         * or if it matched any other token then it should return here to avoid
         * attempting to process tag-specific syntax.
         */
        if ProcessorState::IncompleteInput != self.parser_state
            && ProcessorState::Complete != self.parser_state
            && ProcessorState::MatchedTag != self.parser_state
        {
            return true;
        }

        // Parse all of its attributes.
        while self.parse_next_attribute() {}

        // Ensure that the tag closes before the end of the document.
        if ProcessorState::IncompleteInput == self.parser_state
            || self.bytes_already_parsed >= self.html.len()
        {
            // Does this appropriately clear state (parsed attributes)?
            self.parser_state = ProcessorState::IncompleteInput;
            self.bytes_already_parsed = was_at;

            return false;
        }

        let tag_ends_at = self.html[self.bytes_already_parsed..].find('>');
        if tag_ends_at.is_none() {
            self.parser_state = ProcessorState::IncompleteInput;
            self.bytes_already_parsed = was_at;

            return false;
        }
        self.parser_state = ProcessorState::MatchedTag;
        self.bytes_already_parsed = tag_ends_at.unwrap() + 1;
        self.token_length = Some(
            self.bytes_already_parsed
                - self
                    .token_starts_at
                    .expect("token starts at must be defined here"),
        );

        /*
         * Certain tags require additional processing. The first-letter pre-check
         * avoids unnecessary string allocation when comparing the tag names.
         *
         *  - IFRAME
         *  - LISTING (deprecated)
         *  - NOEMBED (deprecated)
         *  - NOFRAMES (deprecated)
         *  - PRE
         *  - SCRIPT
         *  - STYLE
         *  - TEXTAREA
         *  - TITLE
         *  - XMP (deprecated)
         */
        if self.is_closing_tag.unwrap_or(false)
            || ParsingNamespace::Html != self.parsing_namespace
            || match self.html.as_bytes()[self.token_starts_at.unwrap()] {
                b'i' | b'I' | b'l' | b'L' | b'n' | b'N' | b'p' | b'P' | b's' | b'S' | b't'
                | b'T' | b'x' | b'X' => true,
                _ => false,
            }
        {
            return true;
        }

        let tag_name = self.get_tag();

        /*
         * For LISTING, PRE, and TEXTAREA, the first linefeed of an immediately-following
         * text node is ignored as an authoring convenience.
         *
         * @see static::skip_newline_at
         */
        if tag_name == "LISTING" || tag_name == "PRE" {
            self.skip_newline_at = Some(self.bytes_already_parsed);
            return true;
        }

        /*
         * There are certain elements whose children are not DATA but are instead
         * RCDATA or RAWTEXT. These cannot contain other elements, and the contents
         * are parsed as plaintext, with character references decoded in RCDATA but
         * not in RAWTEXT.
         *
         * These elements are described here as "self-contained" or special atomic
         * elements whose end tag is consumed with the opening tag, and they will
         * contain modifiable text inside of them.
         *
         * Preserve the opening tag pointers, as these will be overwritten
         * when finding the closing tag. They will be reset after finding
         * the closing to tag to point to the opening of the special atomic
         * tag sequence.
         */
        let tag_name_starts_at = self.tag_name_starts_at.unwrap();
        let tag_name_length = self.tag_name_length.unwrap();
        let tag_ends_at = self.token_starts_at.unwrap() + self.token_length.unwrap();
        let attributes = self.attributes;
        let duplicate_attributes = self.duplicate_attributes.clone();

        let found_closer = match tag_name.as_str() {
            "SCRIPT" => self.skip_script_data(),

            "TEXTAREA" | "TITLE" => self.skip_rcdata(&tag_name),

            /*
             * In the browser this list would include the NOSCRIPT element,
             * but the Tag Processor is an environment with the scripting
             * flag disabled, meaning that it needs to descend into the
             * NOSCRIPT element to be able to properly process what will be
             * sent to a browser.
             *
             * Note that this rule makes HTML5 syntax incompatible with XML,
             * because the parsing of this token depends on client application.
             * The NOSCRIPT element cannot be represented in the XHTML syntax.
             */
            "IFRAME" | "NOEMBED" | "NOFRAMES" | "STYLE" | "XMP" => self.skip_rawtext(&tag_name),

            // No other tags should be treated in their entirety here.
            _ => true,
        };

        if !found_closer {
            self.parser_state = ProcessorState::IncompleteInput;
            self.bytes_already_parsed = was_at;
            return false;
        }

        /*
         * The values here look like they reference the opening tag but they reference
         * the closing tag instead. This is why the opening tag values were stored
         * above in a variable. It reads confusingly here, but that's because the
         * functions that skip the contents have moved all the internal cursors past
         * the inner content of the tag.
         */
        self.token_starts_at = Some(was_at);
        self.token_length = Some(self.bytes_already_parsed - self.token_starts_at.unwrap());
        self.text_starts_at = Some(tag_ends_at);
        self.text_length = Some(self.tag_name_starts_at.unwrap() - self.text_starts_at.unwrap());
        self.tag_name_starts_at = Some(tag_name_starts_at);
        self.tag_name_length = Some(tag_name_length);
        self.attributes = attributes;
        self.duplicate_attributes = duplicate_attributes;

        return true;
    }

    /// Applies attribute updates and cleans up once a tag is fully parsed.
    fn after_tag(&mut self) {
        /*
         * There could be lexical updates enqueued for an attribute that
         * also exists on the next tag. In order to avoid conflating the
         * attributes across the two tags, lexical updates with names
         * need to be flushed to raw lexical updates.
         */
        self.class_name_updates_to_attributes_updates();

        /*
         * Purge updates if there are too many. The actual count isn't
         * scientific, but a few values from 100 to a few thousand were
         * tests to find a practically-useful limit.
         *
         * If the update queue grows too big, then the Tag Processor
         * will spend more time iterating through them and lose the
         * efficiency gains of deferring applying them.
         */
        if 1_000 < self.lexical_updates.len() {
            self.get_updated_html();
        }

        if self
            .lexical_updates
            .iter()
            .any(|update| update.start >= self.bytes_already_parsed)
        {
            self.get_updated_html();
        }

        self.token_starts_at = None;
        self.token_length = None;
        self.tag_name_starts_at = None;
        self.tag_name_length = None;
        self.text_starts_at = None;
        self.text_length = None;
        self.is_closing_tag = None;
        self.attributes = ();
        self.comment_type = None;
        self.text_node_classification = TextNodeClassification::Generic;
        self.duplicate_attributes = None;
    }

    fn class_name_updates_to_attributes_updates(&self) {
        unimplemented!()
    }

    fn get_updated_html(&self) {
        unimplemented!()
    }

    fn parse_next_tag(&self) -> bool {
        todo!()
    }

    fn parse_next_attribute(&self) -> bool {
        todo!()
    }

    fn get_tag(&self) -> String {
        todo!()
    }

    fn skip_script_data(&self) -> bool {
        todo!()
    }

    fn skip_rcdata(&self, tag_name: &str) -> bool {
        todo!()
    }

    fn skip_rawtext(&self, tag_name: &str) -> bool {
        todo!()
    }
}

struct TagName(String);
impl Deref for TagName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Default for HtmlProcessor {
    fn default() -> Self {
        Self {
            attributes: (),
            bytes_already_parsed: 0,
            comment_type: None,
            duplicate_attributes: None,
            html: String::new().into_boxed_str(),
            is_closing_tag: None,
            lexical_updates: Vec::new(),
            parser_state: Default::default(),
            parsing_namespace: Default::default(),
            skip_newline_at: None,
            tag_name_length: None,
            tag_name_starts_at: None,
            text_length: None,
            text_node_classification: TextNodeClassification::Generic,
            text_starts_at: None,
            token_length: None,
            token_starts_at: None,
        }
    }
}

#[derive(Default, PartialEq)]
enum ProcessorState {
    #[default]
    Ready,
    Complete,
    IncompleteInput,
    MatchedTag,
    TextNode,
    CDATANode,
    Comment,
    Doctype,
    PresumptuousTag,
    FunkyComment,
}

enum TextNodeClassification {
    Generic,
    NullSequence,
    Whitespace,
}

enum CommentType {
    /**
     * Indicates that a comment was created when encountering abruptly-closed HTML comment.
     *
     * Example:
     *
     *     <!-->
     *     <!--->
     *
     * @since 6.5.0
     */
    AbruptlyClosedComment,

    /**
     * Indicates that a comment would be parsed as a CDATA node,
     * were HTML to allow CDATA nodes outside of foreign content.
     *
     * Example:
     *
     *     <![CDATA[This is a CDATA node.]]>
     *
     * This is an HTML comment, but it looks like a CDATA node.
     *
     * @since 6.5.0
     */
    CdataLookalike,

    /**
     * Indicates that a comment was created when encountering
     * normative HTML comment syntax.
     *
     * Example:
     *
     *     <!-- this is a comment -->
     *
     * @since 6.5.0
     */
    HtmlComment,

    /**
     * Indicates that a comment would be parsed as a Processing
     * Instruction node, were they to exist within HTML.
     *
     * Example:
     *
     *     <?wp __( 'Like' ) ?>
     *
     * This is an HTML comment, but it looks like a CDATA node.
     *
     * @since 6.5.0
     */
    PiNodeLookalike,

    /**
     * Indicates that a comment was created when encountering invalid
     * HTML input, a so-called "bogus comment."
     *
     * Example:
     *
     *     <?nothing special>
     *     <!{nothing special}>
     *
     * @since 6.5.0
     */
    InvalidHtml,
}
