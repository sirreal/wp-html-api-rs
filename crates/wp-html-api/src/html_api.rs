use std::os::unix::process;

pub struct HtmlProcessor {
    html: Box<str>,

    parser_state: ProcessorState,

    bytes_already_parsed: usize,

    lexical_updates: Vec<HtmlTextReplacement>,

    token_starts_at: Option<usize>,
    token_length: Option<usize>,
    tag_name_starts_at: Option<usize>,
    tag_name_length: Option<usize>,
    text_starts_at: usize,
    text_length: usize,
    is_closing_tag: Option<bool>,
    attributes: (),
    comment_type: Option<CommentType>,
    text_node_classification: TextNodeClassification,
    duplicate_attributes: Option<Vec<HtmlSpan>>,
}

struct HtmlTextReplacement {
    start: usize,
    length: usize,
    text: Box<str>,
}

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
        let mut was_at = self.bytes_already_parsed;
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
            if (self.parser_state == ProcessorState::IncompleteInput) {
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
            || "html" != self.parsing_namespace
            || match self.html.as_bytes()[self.token_starts_at.unwrap()] {
                b'i' | b'I' | b'l' | b'L' | b'n' | b'N' | b'p' | b'P' | b's' | b'S' | b't'
                | b'T' | b'x' | b'X' => true,
                _ => false,
            }
        {
            return true;
        }

        todo!()
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

        for update in self.lexical_updates {
            /*
             * Any updates appearing after the cursor should be applied
             * before proceeding, otherwise they may be overlooked.
             */
            if update.start >= self.bytes_already_parsed {
                self.get_updated_html();
                break;
            }
        }

        self.token_starts_at = None;
        self.token_length = None;
        self.tag_name_starts_at = None;
        self.tag_name_length = None;
        self.text_starts_at = 0;
        self.text_length = 0;
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
}

impl Default for HtmlProcessor {
    fn default() -> Self {
        Self {
            html: String::new().into_boxed_str(),
            parser_state: ProcessorState::default(),
            lexical_updates: Vec::new(),
            bytes_already_parsed: 0,

            token_starts_at: None,
            token_length: None,
            tag_name_starts_at: None,
            tag_name_length: None,
            text_starts_at: 0,
            text_length: 0,
            is_closing_tag: None,
            attributes: (),
            comment_type: None,
            text_node_classification: TextNodeClassification::Generic,
            duplicate_attributes: None,
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
    ABRUPTLY_CLOSED_COMMENT,

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
    CDATA_LOOKALIKE,

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
    HTML_COMMENT,

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
    PI_NODE_LOOKALIKE,

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
    INVALID_HTML,
}
