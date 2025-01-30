#![allow(dead_code)]

use std::{collections::HashMap, ops::Deref};

macro_rules! strspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .take_while(|&b| matches!(b, $pattern))
            .count()
    }};
}

macro_rules! strcspn {
    ($expression:expr, $pattern:pat, $offset:expr $(,)?) => {{
        $expression[$offset..]
            .iter()
            .take_while(|&b| !matches!(b, $pattern))
            .count()
    }};
}

pub struct HtmlProcessor {
    attributes: HashMap<Box<[u8]>, AttributeToken>,
    bytes_already_parsed: usize,
    comment_type: Option<CommentType>,
    duplicate_attributes: Option<HashMap<Box<[u8]>, Vec<HtmlSpan>>>,
    html_bytes: Box<[u8]>,
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
        let html_bytes = html.as_bytes().to_vec().into_boxed_slice();
        Self {
            html_bytes,
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

        if self.bytes_already_parsed >= self.html_bytes.len() {
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
            || self.bytes_already_parsed >= self.html_bytes.len()
        {
            // Does this appropriately clear state (parsed attributes)?
            self.parser_state = ProcessorState::IncompleteInput;
            self.bytes_already_parsed = was_at;

            return false;
        }

        let tag_ends_at = strpos(&self.html_bytes, b">", self.bytes_already_parsed);
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
            || match self.html_bytes[self.token_starts_at.unwrap()] {
                b'i' | b'I' | b'l' | b'L' | b'n' | b'N' | b'p' | b'P' | b's' | b'S' | b't'
                | b'T' | b'x' | b'X' => true,
                _ => false,
            }
        {
            return true;
        }

        let tag_name = self.get_tag().unwrap();

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
        let duplicate_attributes = self.duplicate_attributes.clone();

        let found_closer = match tag_name.deref() {
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
        self.attributes = HashMap::new();
        self.comment_type = None;
        self.text_node_classification = TextNodeClassification::Generic;
        self.duplicate_attributes = None;
    }

    fn class_name_updates_to_attributes_updates(&self) {
        // Implement me!
    }

    fn get_updated_html(&self) {
        unimplemented!()
    }

    fn parse_next_tag(&mut self) -> bool {
        self.after_tag();

        let doc_length = self.html_bytes.len();
        let was_at = self.bytes_already_parsed;
        let mut at = was_at;

        while at < self.html_bytes.len() {
            let next_at = strpos(&self.html_bytes, b"<", at);
            if next_at.is_none() {
                break;
            }
            at = at + next_at.unwrap();

            if at > was_at {
                /*
                 * A "<" normally starts a new HTML tag or syntax token, but in cases where the
                 * following character can't produce a valid token, the "<" is instead treated
                 * as plaintext and the parser should skip over it. This avoids a problem when
                 * following earlier practices of typing emoji with text, e.g. "<3". This
                 * should be a heart, not a tag. It's supposed to be rendered, not hidden.
                 *
                 * At this point the parser checks if this is one of those cases and if it is
                 * will continue searching for the next "<" in search of a token boundary.
                 *
                 * @see https://html.spec.whatwg.org/#tag-open-state
                 */
                if matches!( self.html_bytes[at + 1], b'!'| b'/'| b'?'| b'a'..=b'z' | b'A'..=b'Z') {
                    at += 1;
                    continue;
                }

                self.parser_state = ProcessorState::TextNode;
                self.token_starts_at = Some(was_at);
                self.token_length = Some(at - was_at);
                self.text_starts_at = Some(was_at);
                self.text_length = Some(self.token_length.unwrap());
                self.bytes_already_parsed = at;
                return true;
            }

            self.token_starts_at = Some(at);

            if at + 1 < self.html_bytes.len() && b'/' == self.html_bytes[at + 1] {
                self.is_closing_tag = Some(true);
                at += 1;
            } else {
                self.is_closing_tag = Some(false);
            }

            /*
             * HTML tag names must start with [a-zA-Z] otherwise they are not tags.
             * For example, "<3" is rendered as text, not a tag opener. If at least
             * one letter follows the "<" then _it is_ a tag, but if the following
             * character is anything else it _is not a tag_.
             *
             * It's not uncommon to find non-tags starting with `<` in an HTML
             * document, so it's good for performance to make this pre-check before
             * continuing to attempt to parse a tag name.
             *
             * Reference:
             * * https://html.spec.whatwg.org/multipage/parsing.html#data-state
             * * https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
             */
            let tag_name_prefix_length =
                strspn!( self.html_bytes, b'a'..=b'z'|b'A'..=b'Z', at + 1 );

            if tag_name_prefix_length > 0 {
                at += 1;
                self.parser_state = ProcessorState::MatchedTag;
                self.tag_name_starts_at = Some(at);
                self.tag_name_length = Some(
                    tag_name_prefix_length
                        + strcspn!(
                            self.html_bytes,
                            b' ' | b'\t' | 0x0c | b'\r' | b'\n' | b'/' | b'>',
                            at + tag_name_prefix_length
                        ),
                );
                self.bytes_already_parsed = at + self.tag_name_length.unwrap();
                return true;
            }

            /*
             * Abort if no tag is found before the end of
             * the document. There is nothing left to parse.
             */
            if at + 1 >= self.html_bytes.len() {
                self.parser_state = ProcessorState::IncompleteInput;
                return false;
            }

            /*
             * `<!` transitions to markup declaration open state
             * https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
             */
            if !self.is_closing_tag.unwrap_or(false) && b'!' == self.html_bytes[at + 1] {
                /*
                 * `<!--` transitions to a comment state – apply further comment rules.
                 * https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                 */
                if &self.html_bytes[at + 2..at + 4] == b"--" {
                    let mut closer_at = at + 4;
                    // If it's not possible to close the comment then there is nothing more to scan.
                    if self.html_bytes.len() <= closer_at {
                        self.parser_state = ProcessorState::IncompleteInput;
                        return false;
                    }

                    // Abruptly-closed empty comments are a sequence of dashes followed by `>`.
                    let span_of_dashes = strspn!(self.html_bytes, b'-', closer_at);
                    if b'>' == self.html_bytes[closer_at + span_of_dashes] {
                        /*
                         * @todo When implementing `set_modifiable_text()` ensure that updates to this token
                         *       don't break the syntax for short comments, e.g. `<!--->`. Unlike other comment
                         *       and bogus comment syntax, these leave no clear insertion point for text and
                         *       they need to be modified specially in order to contain text. E.g. to store
                         *       `?` as the modifiable text, the `<!--->` needs to become `<!--?-->`, which
                         *       involves inserting an additional `-` into the token after the modifiable text.
                         */
                        self.parser_state = ProcessorState::Comment;
                        self.comment_type = Some(CommentType::AbruptlyClosedComment);
                        self.token_length =
                            Some(closer_at + span_of_dashes + 1 - self.token_starts_at.unwrap());

                        // Only provide modifiable text if the token is long enough to contain it.
                        if span_of_dashes >= 2 {
                            self.comment_type = Some(CommentType::HtmlComment);
                            self.text_starts_at = Some(self.token_starts_at.unwrap() + 4);
                            self.text_length = Some(span_of_dashes - 2);
                        }

                        self.bytes_already_parsed = closer_at + span_of_dashes + 1;
                        return true;
                    }

                    /*
                     * Comments may be closed by either a --> or an invalid --!>.
                     * The first occurrence closes the comment.
                     *
                     * See https://html.spec.whatwg.org/#parse-error-incorrectly-closed-comment
                     */
                    closer_at -= 1; // Pre-increment inside condition below reduces risk of accidental infinite looping.
                    while ({
                        closer_at += 1;
                        closer_at
                    } < self.html_bytes.len())
                    {
                        let next_closer = strpos(&self.html_bytes, b"--", closer_at);
                        if next_closer.is_none() {
                            self.parser_state = ProcessorState::IncompleteInput;
                            return false;
                        }
                        closer_at = next_closer.unwrap();

                        if closer_at + 2 < self.html_bytes.len()
                            && b'>' == self.html_bytes[closer_at + 2]
                        {
                            self.parser_state = ProcessorState::Comment;
                            self.comment_type = Some(CommentType::HtmlComment);
                            self.token_length = Some(closer_at + 3 - self.token_starts_at.unwrap());
                            self.text_starts_at = Some(self.token_starts_at.unwrap() + 4);
                            self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                            self.bytes_already_parsed = closer_at + 3;
                            return true;
                        }

                        if closer_at + 3 < doc_length
                            && b'!' == self.html_bytes[closer_at + 2]
                            && b'>' == self.html_bytes[closer_at + 3]
                        {
                            self.parser_state = ProcessorState::Comment;
                            self.comment_type = Some(CommentType::HtmlComment);
                            self.token_length = Some(closer_at + 4 - self.token_starts_at.unwrap());
                            self.text_starts_at = Some(self.token_starts_at.unwrap() + 4);
                            self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                            self.bytes_already_parsed = closer_at + 4;
                            return true;
                        }
                    }
                }

                /*
                 * `<!DOCTYPE` transitions to DOCTYPE state – skip to the nearest >
                 * These are ASCII-case-insensitive.
                 * https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                 */
                if doc_length > at + 8
                    && matches!(&self.html_bytes[at + 2], b'D' | b'd')
                    && matches!(&self.html_bytes[at + 3], b'O' | b'o')
                    && matches!(&self.html_bytes[at + 4], b'C' | b'c')
                    && matches!(&self.html_bytes[at + 5], b'T' | b't')
                    && matches!(&self.html_bytes[at + 6], b'Y' | b'y')
                    && matches!(&self.html_bytes[at + 7], b'P' | b'p')
                    && matches!(&self.html_bytes[at + 8], b'E' | b'e')
                {
                    let closer_at = strpos(&self.html_bytes, b">", at + 9);
                    if closer_at.is_none() {
                        self.parser_state = ProcessorState::IncompleteInput;
                        return false;
                    }
                    let closer_at = closer_at.unwrap();

                    let token_starts_at = self.token_starts_at.unwrap();
                    self.parser_state = ProcessorState::Doctype;
                    self.token_length = Some(closer_at + 1 - token_starts_at);
                    self.text_starts_at = Some(token_starts_at + 9);
                    self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                    self.bytes_already_parsed = closer_at + 1;
                    return true;
                }

                if self.parsing_namespace == ParsingNamespace::Html
                    && doc_length > at + 8
                    && &self.html_bytes[at + 2..=at + 8] == b"[CDATA["
                {
                    let closer_at = strpos(&self.html_bytes, b"]]>", at + 9);
                    if closer_at.is_none() {
                        self.parser_state = ProcessorState::IncompleteInput;
                        return false;
                    }
                    let closer_at = closer_at.unwrap();

                    self.parser_state = ProcessorState::CDATANode;
                    self.text_starts_at = Some(at + 9);
                    self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                    self.token_length = Some(closer_at + 3 - self.token_starts_at.unwrap());
                    self.bytes_already_parsed = closer_at + 3;
                    return true;
                }

                /*
                 * Anything else here is an incorrectly-opened comment and transitions
                 * to the bogus comment state - skip to the nearest >. If no closer is
                 * found then the HTML was truncated inside the markup declaration.
                 */
                let closer_at = strpos(&self.html_bytes, b">", at + 1);
                if closer_at.is_none() {
                    self.parser_state = ProcessorState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ProcessorState::Comment;
                self.comment_type = Some(CommentType::InvalidHtml);
                self.token_length = Some(closer_at + 1 - self.token_starts_at.unwrap());
                self.text_starts_at = Some(self.token_starts_at.unwrap() + 2);
                self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                self.bytes_already_parsed = closer_at + 1;

                /*
                 * Identify nodes that would be CDATA if HTML had CDATA sections.
                 *
                 * This section must occur after identifying the bogus comment end
                 * because in an HTML parser it will span to the nearest `>`, even
                 * if there's no `]]>` as would be required in an XML document. It
                 * is therefore not possible to parse a CDATA section containing
                 * a `>` in the HTML syntax.
                 *
                 * Inside foreign elements there is a discrepancy between browsers
                 * and the specification on this.
                 *
                 * @todo Track whether the Tag Processor is inside a foreign element
                 *       and require the proper closing `]]>` in those cases.
                 */
                if self.token_length.unwrap() >= 10
                    && b'[' == self.html_bytes[self.token_starts_at.unwrap() + 2]
                    && b'C' == self.html_bytes[self.token_starts_at.unwrap() + 3]
                    && b'D' == self.html_bytes[self.token_starts_at.unwrap() + 4]
                    && b'A' == self.html_bytes[self.token_starts_at.unwrap() + 5]
                    && b'T' == self.html_bytes[self.token_starts_at.unwrap() + 6]
                    && b'A' == self.html_bytes[self.token_starts_at.unwrap() + 7]
                    && b'[' == self.html_bytes[self.token_starts_at.unwrap() + 8]
                    && b']' == self.html_bytes[closer_at - 1]
                    && b']' == self.html_bytes[closer_at - 2]
                {
                    self.parser_state = ProcessorState::Comment;
                    self.comment_type = Some(CommentType::CdataLookalike);
                    self.text_starts_at = Some(self.text_starts_at.unwrap() + 7);
                    self.text_length = Some(self.text_length.unwrap() - 9);
                }

                return true;
            }

            /*
             * </> is a missing end tag name, which is ignored.
             *
             * This was also known as the "presumptuous empty tag"
             * in early discussions as it was proposed to close
             * the nearest previous opening tag.
             *
             * See https://html.spec.whatwg.org/#parse-error-missing-end-tag-name
             */
            if b'>' == self.html_bytes[at + 1] {
                // `<>` is interpreted as plaintext.
                if !self.is_closing_tag.unwrap() {
                    at += 1;
                    continue;
                }

                self.parser_state = ProcessorState::PresumptuousTag;
                self.token_length = Some(at + 2 - self.token_starts_at.unwrap());
                self.bytes_already_parsed = at + 2;
                return true;
            }

            /*
             * `<?` transitions to a bogus comment state – skip to the nearest >
             * See https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
             */
            if !self.is_closing_tag.unwrap() && b'?' == self.html_bytes[at + 1] {
                let closer_at = strpos(&self.html_bytes, b">", at + 2);
                if closer_at.is_none() {
                    self.parser_state = ProcessorState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ProcessorState::Comment;
                self.comment_type = Some(CommentType::InvalidHtml);
                self.token_length = Some(closer_at + 1 - self.token_starts_at.unwrap());
                self.text_starts_at = Some(self.token_starts_at.unwrap() + 2);
                self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                self.bytes_already_parsed = closer_at + 1;

                /*
                 * Identify a Processing Instruction node were HTML to have them.
                 *
                 * This section must occur after identifying the bogus comment end
                 * because in an HTML parser it will span to the nearest `>`, even
                 * if there's no `?>` as would be required in an XML document. It
                 * is therefore not possible to parse a Processing Instruction node
                 * containing a `>` in the HTML syntax.
                 *
                 * XML allows for more target names, but this code only identifies
                 * those with ASCII-representable target names. This means that it
                 * may identify some Processing Instruction nodes as bogus comments,
                 * but it will not misinterpret the HTML structure. By limiting the
                 * identification to these target names the Tag Processor can avoid
                 * the need to start parsing UTF-8 sequences.
                 *
                 * > NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] | [#xF8-#x2FF] |
                 *                     [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] |
                 *                     [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] |
                 *                     [#x10000-#xEFFFF]
                 * > NameChar      ::= NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
                 *
                 * @todo Processing instruction nodes in SGML may contain any kind of markup. XML defines a
                 *       special case with `<?xml ... ?>` syntax, but the `?` is part of the bogus comment.
                 *
                 * @see https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-PITarget
                 */
                if self.token_length.unwrap() >= 5 && b'?' == self.html_bytes[closer_at - 1] {
                    let comment_text = substr(
                        &self.html_bytes,
                        self.token_starts_at.unwrap() + 2,
                        self.token_length.unwrap() - 4,
                    );
                    let mut pi_target_length =
                        strspn!( comment_text, b'a'..=b'z'|b'A'..b'Z'|b':'|b'_', 0 );

                    if 0 < pi_target_length {
                        pi_target_length += strspn!( comment_text, b'a'..=b'z'|b'A'..b'Z'|b':'|b'_'|b'-'|b'.', pi_target_length );

                        self.comment_type = Some(CommentType::PiNodeLookalike);
                        self.tag_name_starts_at = Some(self.token_starts_at.unwrap() + 2);
                        self.tag_name_length = Some(pi_target_length);
                        self.text_starts_at = Some(self.text_starts_at.unwrap() + pi_target_length);
                        self.text_length = Some(self.text_length.unwrap() - (pi_target_length + 1));
                    }
                }

                return true;
            }

            /*
             * If a non-alpha starts the tag name in a tag closer it's a comment.
             * Find the first `>`, which closes the comment.
             *
             * This parser classifies these particular comments as special "funky comments"
             * which are made available for further processing.
             *
             * See https://html.spec.whatwg.org/#parse-error-invalid-first-character-of-tag-name
             */
            if self.is_closing_tag.unwrap() {
                // No chance of finding a closer.
                if at + 3 > doc_length {
                    self.parser_state = ProcessorState::IncompleteInput;
                    return false;
                }

                let closer_at = strpos(&self.html_bytes, b">", at + 2);
                if closer_at.is_none() {
                    self.parser_state = ProcessorState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ProcessorState::FunkyComment;
                self.token_length = Some(closer_at + 1 - self.token_starts_at.unwrap());
                self.text_starts_at = Some(self.token_starts_at.unwrap() + 2);
                self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                self.bytes_already_parsed = closer_at + 1;
                return true;
            }

            at += 1;
        }

        /*
         * This does not imply an incomplete parse; it indicates that there
         * can be nothing left in the document other than a #text node.
         */
        self.parser_state = ProcessorState::TextNode;
        self.token_starts_at = Some(was_at);
        self.token_length = Some(doc_length - was_at);
        self.text_starts_at = Some(was_at);
        self.text_length = Some(self.token_length.unwrap());
        self.bytes_already_parsed = doc_length;

        true
    }

    fn parse_next_attribute(&mut self) -> bool {
        let doc_length = self.html_bytes.len();

        // Skip whitespace and slashes.
        self.bytes_already_parsed += strspn!(
            &self.html_bytes,
            b' ' | b'\t' | 0x0c | b'\r' | b'\n' | b'/',
            self.bytes_already_parsed
        );
        if self.bytes_already_parsed >= doc_length {
            self.parser_state = ProcessorState::IncompleteInput;
            return false;
        }

        /*
         * Treat the equal sign as a part of the attribute
         * name if it is the first encountered byte.
         *
         * @see https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
         */
        let name_length = if b'=' == self.html_bytes[self.bytes_already_parsed] {
            1 + strcspn!(
                self.html_bytes,
                b'=' | b'/' | b'>' | b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                self.bytes_already_parsed + 1
            )
        } else {
            strcspn!(
                self.html_bytes,
                b'=' | b'/' | b'>' | b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                self.bytes_already_parsed
            )
        };

        // No attribute, just tag closer.
        if 0 == name_length || self.bytes_already_parsed + name_length >= doc_length {
            return false;
        }

        let attribute_start = self.bytes_already_parsed;
        let attribute_name = substr(&self.html_bytes, attribute_start, name_length);
        self.bytes_already_parsed += name_length;
        if self.bytes_already_parsed >= doc_length {
            self.parser_state = ProcessorState::IncompleteInput;
            return false;
        }

        self.skip_whitespace();
        if self.bytes_already_parsed >= doc_length {
            self.parser_state = ProcessorState::IncompleteInput;
            return false;
        }

        let has_value = b'=' == self.html_bytes[self.bytes_already_parsed];
        let (value_start, value_length, attribute_end) = if has_value {
            self.bytes_already_parsed += 1;
            self.skip_whitespace();
            if self.bytes_already_parsed >= doc_length {
                self.parser_state = ProcessorState::IncompleteInput;
                return false;
            }

            match self.html_bytes[self.bytes_already_parsed] {
                quote @ (b'\'' | b'"') => {
                    let value_start = self.bytes_already_parsed + 1;
                    let end_quote_at = strpos(&self.html_bytes, &[quote], value_start);
                    let end_quote_at = end_quote_at.unwrap_or(doc_length);
                    let value_length = end_quote_at - value_start;
                    let attribute_end = end_quote_at + 1;
                    self.bytes_already_parsed = attribute_end;
                    (value_start, value_length, attribute_end)
                }

                _ => {
                    let value_start = self.bytes_already_parsed;
                    let value_length = strcspn!(
                        self.html_bytes,
                        b'>' | b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                        value_start
                    );
                    let attribute_end = value_start + value_length;
                    self.bytes_already_parsed = attribute_end;
                    (value_start, value_length, attribute_end)
                }
            }
        } else {
            let value_start = self.bytes_already_parsed;
            let value_length = 0;
            let attribute_end = attribute_start + name_length;
            (value_start, value_length, attribute_end)
        };

        if attribute_end >= doc_length {
            self.parser_state = ProcessorState::IncompleteInput;
            return false;
        }

        if self.is_closing_tag.unwrap() {
            return true;
        }

        /*
         * > There must never be two or more attributes on
         * > the same start tag whose names are an ASCII
         * > case-insensitive match for each other.
         *     - HTML 5 spec
         *
         * @see https://html.spec.whatwg.org/multipage/syntax.html#attributes-2:ascii-case-insensitive
         */
        let comparable_name = attribute_name.to_ascii_lowercase().into_boxed_slice();

        // If an attribute is listed many times, only use the first declaration and ignore the rest.
        if !self.attributes.contains_key(&comparable_name) {
            let attribute_token = AttributeToken {
                name: attribute_name.to_vec().into_boxed_slice(),
                value_starts_at: value_start,
                value_length,
                start: attribute_start,
                length: attribute_end - attribute_start,
                is_true: !has_value,
            };
            self.attributes.insert(comparable_name, attribute_token);
            return true;
        }

        /*
         * Track the duplicate attributes so if we remove it, all disappear together.
         *
         * While `$this->duplicated_attributes` could always be stored as an `array()`,
         * which would simplify the logic here, storing a `null` and only allocating
         * an array when encountering duplicates avoids needless allocations in the
         * normative case of parsing tags with no duplicate attributes.
         */
        let duplicate_span = HtmlSpan {
            start: attribute_start,
            length: attribute_end - attribute_start,
        };
        if self.duplicate_attributes.is_none() {
            let mut duplicate_attributes = HashMap::new();
            duplicate_attributes.insert(comparable_name, vec![duplicate_span]);
            self.duplicate_attributes = Some(duplicate_attributes);
        } else {
            let dupes = self.duplicate_attributes.as_mut().unwrap();
            if let Some(v) = dupes.get_mut(&comparable_name) {
                v.push(duplicate_span);
            } else {
                dupes.insert(comparable_name, vec![duplicate_span]);
            }
        }

        return true;
    }

    pub fn get_tag(&self) -> Option<TagName> {
        self.tag_name_starts_at
            .and_then(|start| {
                self.tag_name_length
                    .and_then(|length| match self.parser_state {
                        ProcessorState::MatchedTag => String::from_utf8(
                            substr(&self.html_bytes, start, length).to_ascii_uppercase(),
                        )
                        .ok()
                        .map(|s| s.into_boxed_str()),
                        ProcessorState::Comment => self
                            .comment_type
                            .filter(|ct| ct == &CommentType::PiNodeLookalike)
                            .and_then(|_| {
                                String::from_utf8(substr(&self.html_bytes, start, length).to_vec())
                                    .ok()
                                    .map(|s| s.into_boxed_str())
                            }),
                        _ => None,
                    })
            })
            .map(|s| TagName(s))
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
    pub fn get_token_type(&self) -> Option<TokenType> {
        match self.parser_state {
            ProcessorState::MatchedTag => Some(TokenType::Tag),
            ProcessorState::Doctype => Some(TokenType::Doctype),
            ProcessorState::TextNode => Some(TokenType::Text),
            ProcessorState::CDATANode => Some(TokenType::CdataSection),
            ProcessorState::Comment => Some(TokenType::Comment),
            ProcessorState::PresumptuousTag => Some(TokenType::PresumptuousTag),
            ProcessorState::FunkyComment => Some(TokenType::FunkyComment),

            ProcessorState::Ready | ProcessorState::Complete | ProcessorState::IncompleteInput => {
                None
            }
        }
    }

    pub fn get_token_name(&self) -> Option<Box<str>> {
        match self.parser_state {
            ProcessorState::MatchedTag => self.get_tag().map(|t| t.0),
            ProcessorState::Doctype => Some("html".into()),
            _ => self.get_token_type().map(|t| t.into()),
        }
    }

    fn skip_script_data(&mut self) -> bool {
        let mut state = ScriptState::Unescaped;
        let doc_length = self.html_bytes.len();
        let mut at = self.bytes_already_parsed;

        while at < doc_length {
            at += strcspn!(self.html_bytes, b'-' | b'<', at);

            /*
             * For all script states a "-->"  transitions
             * back into the normal unescaped script mode,
             * even if that's the current state.
             */
            if at + 2 < doc_length
                && self.html_bytes[at] == b'-'
                && self.html_bytes[at + 1] == b'-'
                && self.html_bytes[at + 2] == b'>'
            {
                at += 3;
                state = ScriptState::Unescaped;
                continue;
            }

            if at + 1 >= doc_length {
                return false;
            }

            /*
             * Everything of interest past here starts with "<".
             * Check this character and advance position regardless.
             */
            at += 1;
            if self.html_bytes[at - 1] != b'<' {
                continue;
            }

            /*
             * Unlike with "-->", the "<!--" only transitions
             * into the escaped mode if not already there.
             *
             * Inside the escaped modes it will be ignored; and
             * should never break out of the double-escaped
             * mode and back into the escaped mode.
             *
             * While this requires a mode change, it does not
             * impact the parsing otherwise, so continue
             * parsing after updating the state.
             */
            if at + 2 < doc_length
                && self.html_bytes[at] == b'!'
                && self.html_bytes[at + 1] == b'-'
                && self.html_bytes[at + 2] == b'-'
            {
                at += 3;
                if state == ScriptState::Unescaped {
                    state = ScriptState::Escaped;
                }
                continue;
            }

            let is_closing = if self.html_bytes[at] == b'/' {
                let closer_potentially_starts_at = at - 1;
                at += 1;
                Some(closer_potentially_starts_at)
            } else {
                None
            };

            /*
             * At this point the only remaining state-changes occur with the
             * <script> and </script> tags; unless one of these appears next,
             * proceed scanning to the next potential token in the text.
             */
            if !(at + 6 < doc_length
                && (b's' == self.html_bytes[at] || b'S' == self.html_bytes[at])
                && (b'c' == self.html_bytes[at + 1] || b'C' == self.html_bytes[at + 1])
                && (b'r' == self.html_bytes[at + 2] || b'R' == self.html_bytes[at + 2])
                && (b'i' == self.html_bytes[at + 3] || b'I' == self.html_bytes[at + 3])
                && (b'p' == self.html_bytes[at + 4] || b'P' == self.html_bytes[at + 4])
                && (b't' == self.html_bytes[at + 5] || b'T' == self.html_bytes[at + 5]))
            {
                at += 1;
                continue;
            }

            /*
             * Ensure that the script tag terminates to avoid matching on
             * substrings of a non-match. For example, the sequence
             * "<script123" should not end a script region even though
             * "<script" is found within the text.
             */
            if at + 6 >= doc_length {
                continue;
            }
            at += 6;
            if !matches!(
                self.html_bytes[at],
                b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>'
            ) {
                at += 1;
                continue;
            }

            if state == ScriptState::Escaped && is_closing.is_none() {
                state = ScriptState::DoubleEscaped;
                continue;
            }

            if state == ScriptState::DoubleEscaped && is_closing.is_some() {
                state = ScriptState::Escaped;
                continue;
            }

            if let Some(closer_starts_at) = is_closing {
                self.bytes_already_parsed = closer_starts_at;
                self.tag_name_starts_at = Some(closer_starts_at);
                if (self.bytes_already_parsed >= doc_length) {
                    return false;
                }

                while self.parse_next_attribute() {}

                if (self.bytes_already_parsed >= doc_length) {
                    self.parser_state = ProcessorState::IncompleteInput;
                    return false;
                }

                if b'>' == self.html_bytes[self.bytes_already_parsed] {
                    self.bytes_already_parsed += 1;
                    return true;
                }
            }

            at += 1;
        }

        false
    }

    fn skip_rcdata(&self, tag_name: &str) -> bool {
        todo!()
    }

    fn skip_rawtext(&self, tag_name: &str) -> bool {
        todo!()
    }

    fn skip_whitespace(&self) -> () {
        todo!()
    }
}

pub(crate) struct TagName(Box<str>);
impl Deref for TagName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.0.deref() == *other
    }
}
impl Into<Box<str>> for TagName {
    fn into(self) -> Box<str> {
        self.0
    }
}
impl Into<String> for TagName {
    fn into(self) -> String {
        self.0.into()
    }
}

impl Default for HtmlProcessor {
    fn default() -> Self {
        Self {
            attributes: HashMap::new(),
            bytes_already_parsed: 0,
            comment_type: None,
            duplicate_attributes: None,
            html_bytes: Box::new([]),
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

#[derive(Clone, Copy, PartialEq)]
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

fn substr(s: &[u8], offset: usize, length: usize) -> &[u8] {
    &s[offset..offset + length]
}

fn strpos(s: &[u8], pattern: &[u8], offset: usize) -> Option<usize> {
    let window_size = pattern.len();
    s[offset..]
        .windows(window_size)
        .position(|bytes| bytes == pattern)
}

pub(crate) enum TokenType {
    Tag,
    Text,
    CdataSection,
    Comment,
    Doctype,
    PresumptuousTag,
    FunkyComment,
}

impl Into<String> for TokenType {
    fn into(self) -> String {
        match self {
            TokenType::Tag => "#tag".into(),
            TokenType::Text => "#text".into(),
            TokenType::CdataSection => "#cdata-section".into(),
            TokenType::Comment => "#comment".into(),
            TokenType::Doctype => "#doctype".into(),
            TokenType::PresumptuousTag => "#presumptuous-tag".into(),
            TokenType::FunkyComment => "#funky-comment".into(),
        }
    }
}

impl Into<Box<str>> for TokenType {
    fn into(self) -> Box<str> {
        match self {
            TokenType::Tag => "#tag".into(),
            TokenType::Text => "#text".into(),
            TokenType::CdataSection => "#cdata-section".into(),
            TokenType::Comment => "#comment".into(),
            TokenType::Doctype => "#doctype".into(),
            TokenType::PresumptuousTag => "#presumptuous-tag".into(),
            TokenType::FunkyComment => "#funky-comment".into(),
        }
    }
}

struct AttributeToken {
    /// The attribute name.
    pub name: Box<[u8]>,

    /// The byte offset where the attribute value starts.
    pub value_starts_at: usize,

    /// The byte length of the attribute value
    pub value_length: usize,

    /// The byte offset where the attribute name starts.
    pub start: usize,

    /// Byte length of text spanning the attribute inside a tag.
    ///
    /// This span starts at the first character of the attribute name
    /// and it ends after one of three cases:
    ///
    ///  - at the end of the attribute name for boolean attributes.
    ///  - at the end of the value for unquoted attributes.
    ///  - at the final single or double quote for quoted attributes.
    ///
    /// Example:
    ///
    ///     <div class="post">
    ///          ------------ length is 12, including quotes
    ///
    ///     <input type="checked" checked id="selector">
    ///                           ------- length is 6
    ///
    ///     <a rel=noopener>
    ///        ------------ length is 11
    ///
    pub length: usize,

    /// Whether the attribute is a boolean attribute with value `true`.
    pub is_true: bool,
}

#[derive(PartialEq)]
enum ScriptState {
    Unescaped,
    Escaped,
    DoubleEscaped,
}
