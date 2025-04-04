#![allow(dead_code, unused_variables)]

use rustc_hash::FxHashMap;

use crate::{
    attributes::qualified_attribute_name,
    compat_mode::CompatMode,
    doctype::HtmlDoctypeInfo,
    str_fns::{stripos, strpos, strpos_byte, substr},
};

use super::tag_name::TagName;

use std::collections::{BTreeSet, HashMap};

const MAX_BOOKMARKS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum TagClosers {
    Skip,
    Visit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NextTagQuery {
    pub tag_name: Option<TagName>,
    pub match_offset: usize,
    pub class_name: Option<String>,
    pub tag_closers: TagClosers,
}
impl Default for NextTagQuery {
    fn default() -> Self {
        Self {
            tag_name: None,
            match_offset: 1,
            class_name: None,
            tag_closers: TagClosers::Skip,
        }
    }
}

pub struct TagProcessor {
    attributes: Vec<AttributeToken>,
    pub bytes_already_parsed: usize,
    comment_type: Option<CommentType>,
    pub html_bytes: Box<[u8]>,
    is_closing_tag: Option<bool>,
    lexical_updates: Vec<HtmlTextReplacement>,
    pub(crate) parser_state: ParserState,
    parsing_namespace: ParsingNamespace,
    skip_newline_at: Option<usize>,
    tag_name_length: Option<usize>,
    tag_name_starts_at: Option<usize>,
    text_length: Option<usize>,
    pub(crate) text_node_classification: TextNodeClassification,
    text_starts_at: Option<usize>,
    token_length: Option<usize>,
    token_starts_at: Option<usize>,

    /// Indicates if the document is in quirks mode or no-quirks mode.
    ///
    ///  Impact on HTML parsing:
    ///
    ///   - In `NO_QUIRKS_MODE` (also known as "standard mode"):
    ///       - CSS class and ID selectors match byte-for-byte (case-sensitively).
    ///       - A TABLE start tag `<table>` implicitly closes any open `P` element.
    ///
    ///   - In `QUIRKS_MODE`:
    ///       - CSS class and ID selectors match match in an ASCII case-insensitive manner.
    ///       - A TABLE start tag `<table>` opens a `TABLE` element as a child of a `P`
    ///         element if one is open.
    ///
    /// Quirks and no-quirks mode are thus mostly about styling, but have an impact when
    /// tables are found inside paragraph elements.
    ///
    /// @see self::QUIRKS_MODE
    /// @see self::NO_QUIRKS_MODE
    pub(crate) compat_mode: CompatMode,

    pub(crate) bookmarks: HashMap<Box<str>, HtmlSpan>,
    pub(crate) internal_bookmarks: FxHashMap<u32, HtmlSpan>,
}

#[derive(Default, PartialEq, Debug, Clone)]
pub enum ParsingNamespace {
    #[default]
    Html,
    Svg,
    MathML,
}
impl From<ParsingNamespace> for String {
    fn from(val: ParsingNamespace) -> Self {
        (&val).into()
    }
}
impl From<&ParsingNamespace> for String {
    fn from(val: &ParsingNamespace) -> Self {
        match val {
            ParsingNamespace::Html => "html",
            ParsingNamespace::Svg => "svg",
            ParsingNamespace::MathML => "math",
        }
        .to_string()
    }
}

struct HtmlTextReplacement {
    start: usize,
    length: usize,
    text: Box<str>,
}

#[derive(Clone)]
pub(crate) struct HtmlSpan {
    pub(crate) start: usize,
    pub(crate) length: usize,
}
impl HtmlSpan {
    pub fn new(start: usize, length: usize) -> Self {
        Self { start, length }
    }
}

impl HtmlTextReplacement {
    pub fn new(start: usize, length: usize, text: &str) -> Self {
        Self {
            start,
            length,
            text: text.into(),
        }
    }
}

impl TagProcessor {
    pub fn new(html: &[u8]) -> Self {
        let html_bytes = html.into();
        Self {
            html_bytes,
            ..Default::default()
        }
    }

    /// Finds the next token in the HTML document.
    ///
    /// An HTML document can be viewed as a stream of tokens,
    /// where tokens are things like HTML tags, HTML comments,
    /// text nodes, etc. This method finds the next token in
    /// the HTML document and returns whether it found one.
    ///
    /// If it starts parsing a token and reaches the end of the
    /// document then it will seek to the start of the last
    /// token and pause, returning `false` to indicate that it
    /// failed to find a complete token.
    ///
    /// Possible token types, based on the HTML specification:
    ///
    ///  - an HTML tag, whether opening, closing, or void.
    ///  - a text node - the plaintext inside tags.
    ///  - an HTML comment.
    ///  - a DOCTYPE declaration.
    ///  - a processing instruction, e.g. `<?xml version="1.0" ?>`.
    ///
    /// The Tag Processor currently only supports the tag token.
    ///
    /// @return bool Whether a token was parsed.
    pub fn next_token(&mut self) -> bool {
        self.base_class_next_token()
    }

    /// Finds the next tag matching the $query.
    ///
    /// @param array|string|null $query {
    ///     Optional. Which tag name to find, having which class, etc. Default is to find any tag.
    ///
    ///     @type string|null $tag_name     Which tag to find, or `null` for "any tag."
    ///     @type int|null    $match_offset Find the Nth tag matching all search criteria.
    ///                                     1 for "first" tag, 3 for "third," etc.
    ///                                     Defaults to first tag.
    ///     @type string|null $class_name   Tag must contain this whole class name to match.
    ///     @type string|null $tag_closers  "visit" or "skip": whether to stop on tag closers, e.g. </div>.
    /// }
    /// @return bool Whether a tag was matched.
    pub fn next_tag(&mut self, query: Option<NextTagQuery>) -> bool {
        let query = query.unwrap_or_default();
        let mut already_found = 0;

        loop {
            if !self.next_token() {
                return false;
            }

            if self.parser_state != ParserState::MatchedTag {
                continue;
            }

            if self.matches(&query) {
                already_found += 1
            }

            // Check if we've found enough matches
            if already_found >= query.match_offset {
                return true;
            }
        }
    }

    /// Checks whether a given tag and its attributes match the search criteria.
    ///
    /// @return bool Whether the given tag and its attribute match the search criteria.
    fn matches(&self, query: &NextTagQuery) -> bool {
        let stop_on_tag_closers = query.tag_closers == TagClosers::Visit;
        if self.is_closing_tag.unwrap_or(false) && !stop_on_tag_closers {
            return false;
        }

        // Does the tag name match the requested tag name in a case-insensitive manner?
        if let Some(sought_tag) = &query.tag_name {
            if let Some(current_tag) = &self.get_tag() {
                if current_tag != sought_tag {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(class_name) = &query.class_name {
            if !self.has_class(class_name).unwrap_or(false) {
                return false;
            }
        }

        true
    }

    /// Internal method which finds the next token in the HTML document.
    ///
    /// This method is a protected internal function which implements the logic for
    /// finding the next token in a document. It exists so that the parser can update
    /// its state without affecting the location of the cursor in the document and
    /// without triggering subclass methods for things like `next_token()`, e.g. when
    /// applying patches before searching for the next token.
    ///
    /// @return bool Whether a token was parsed.
    fn base_class_next_token(&mut self) -> bool {
        let was_at = self.bytes_already_parsed;
        self.after_tag();

        if ParserState::Complete == self.parser_state
            || ParserState::IncompleteInput == self.parser_state
        {
            return false;
        }

        /*
         * The next step in the parsing loop determines the parsing state;
         * clear it so that state doesn't linger from the previous step.
         */
        self.parser_state = ParserState::Ready;

        if self.bytes_already_parsed >= self.html_bytes.len() {
            self.parser_state = ParserState::Complete;
            return false;
        }

        // Find the next tag if it exists.
        if !self.parse_next_tag() {
            if self.parser_state == ParserState::IncompleteInput {
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
        if ParserState::IncompleteInput != self.parser_state
            && ParserState::Complete != self.parser_state
            && ParserState::MatchedTag != self.parser_state
        {
            return true;
        }

        // Parse all of its attributes.
        while self.parse_next_attribute() {}

        // Ensure that the tag closes before the end of the document.
        if ParserState::IncompleteInput == self.parser_state
            || self.bytes_already_parsed >= self.html_bytes.len()
        {
            // Does this appropriately clear state (parsed attributes)?
            self.parser_state = ParserState::IncompleteInput;
            self.bytes_already_parsed = was_at;

            return false;
        }

        let tag_ends_at = strpos_byte(&self.html_bytes, b'>', self.bytes_already_parsed);
        if tag_ends_at.is_none() {
            self.parser_state = ParserState::IncompleteInput;
            self.bytes_already_parsed = was_at;
            return false;
        }
        let tag_ends_at = tag_ends_at.unwrap();
        self.parser_state = ParserState::MatchedTag;
        self.bytes_already_parsed = tag_ends_at + 1;
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
            || matches!(
                self.html_bytes[self.token_starts_at.unwrap()],
                b'i' | b'I'
                    | b'l'
                    | b'L'
                    | b'n'
                    | b'N'
                    | b'p'
                    | b'P'
                    | b's'
                    | b'S'
                    | b't'
                    | b'T'
                    | b'x'
                    | b'X'
            )
        {
            return true;
        }

        let tag = self.get_tag().unwrap();

        /*
         * For LISTING, PRE, and TEXTAREA, the first linefeed of an immediately-following
         * text node is ignored as an authoring convenience.
         *
         * @see static::skip_newline_at
         */
        if tag == TagName::LISTING || tag == TagName::PRE {
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
        let attributes = Vec::clone(&self.attributes);

        let found_closer = match tag {
            TagName::SCRIPT => self.skip_script_data(),

            TagName::TEXTAREA | TagName::TITLE => self.skip_rcdata(&tag),

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
            TagName::IFRAME
            | TagName::NOEMBED
            | TagName::NOFRAMES
            | TagName::STYLE
            | TagName::XMP => self.skip_rawtext(&tag),

            // No other tags should be treated in their entirety here.
            _ => return true,
        };

        if !found_closer {
            self.parser_state = ParserState::IncompleteInput;
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
        self.attributes = attributes;
        self.tag_name_length = Some(tag_name_length);

        true
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
        self.attributes.clear();
        self.comment_type = None;
        self.text_node_classification = TextNodeClassification::Generic;
    }

    fn class_name_updates_to_attributes_updates(&self) {
        // Implement me!
    }

    /// Returns the string representation of the HTML Tag Processor.
    ///
    /// @return string The processed HTML.
    pub fn get_updated_html(&self) -> Box<[u8]> {
        self.html_bytes.clone()
    }

    fn parse_next_tag(&mut self) -> bool {
        self.after_tag();

        let doc_length = self.html_bytes.len();
        let was_at = self.bytes_already_parsed;
        let mut at = was_at;

        while at < doc_length {
            let next_at = strpos_byte(&self.html_bytes, b'<', at);
            if next_at.is_none() {
                break;
            }
            at = next_at.unwrap();

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
                if !matches!( self.html_bytes[at + 1], b'!'| b'/'| b'?'| b'a'..=b'z' | b'A'..=b'Z')
                {
                    at += 1;
                    continue;
                }

                self.parser_state = ParserState::TextNode;
                self.token_starts_at = Some(was_at);
                self.token_length = Some(at - was_at);
                self.text_starts_at = Some(was_at);
                self.text_length = Some(self.token_length.unwrap());
                self.bytes_already_parsed = at;
                return true;
            }

            self.token_starts_at = Some(at);

            if at + 1 < doc_length && b'/' == self.html_bytes[at + 1] {
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
                self.parser_state = ParserState::MatchedTag;
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
                self.parser_state = ParserState::IncompleteInput;
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
                if self.html_bytes.len() > at + 3 && &self.html_bytes[at + 2..at + 4] == b"--" {
                    let mut closer_at = at + 4;
                    // If it's not possible to close the comment then there is nothing more to scan.
                    if self.html_bytes.len() <= closer_at {
                        self.parser_state = ParserState::IncompleteInput;
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
                        self.parser_state = ParserState::Comment;
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
                            self.parser_state = ParserState::IncompleteInput;
                            return false;
                        }
                        closer_at = next_closer.unwrap();

                        if closer_at + 2 < self.html_bytes.len()
                            && b'>' == self.html_bytes[closer_at + 2]
                        {
                            self.parser_state = ParserState::Comment;
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
                            self.parser_state = ParserState::Comment;
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
                    let closer_at = strpos_byte(&self.html_bytes, b'>', at + 9);
                    if closer_at.is_none() {
                        self.parser_state = ParserState::IncompleteInput;
                        return false;
                    }
                    let closer_at = closer_at.unwrap();

                    let token_starts_at = self.token_starts_at.unwrap();
                    self.parser_state = ParserState::Doctype;
                    self.token_length = Some(closer_at + 1 - token_starts_at);
                    self.text_starts_at = Some(token_starts_at + 9);
                    self.text_length = Some(closer_at - self.text_starts_at.unwrap());
                    self.bytes_already_parsed = closer_at + 1;
                    return true;
                }

                if self.parsing_namespace != ParsingNamespace::Html
                    && doc_length > at + 8
                    && &self.html_bytes[at + 2..=at + 8] == b"[CDATA["
                {
                    let closer_at = strpos(&self.html_bytes, b"]]>", at + 9);
                    if closer_at.is_none() {
                        self.parser_state = ParserState::IncompleteInput;
                        return false;
                    }
                    let closer_at = closer_at.unwrap();

                    self.parser_state = ParserState::CDATANode;
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
                let closer_at = strpos_byte(&self.html_bytes, b'>', at + 1);
                if closer_at.is_none() {
                    self.parser_state = ParserState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ParserState::Comment;
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
                    self.parser_state = ParserState::Comment;
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

                self.parser_state = ParserState::PresumptuousTag;
                self.token_length = Some(at + 2 - self.token_starts_at.unwrap());
                self.bytes_already_parsed = at + 2;
                return true;
            }

            /*
             * `<?` transitions to a bogus comment state – skip to the nearest >
             * See https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
             */
            if !self.is_closing_tag.unwrap() && b'?' == self.html_bytes[at + 1] {
                let closer_at = strpos_byte(&self.html_bytes, b'>', at + 2);
                if closer_at.is_none() {
                    self.parser_state = ParserState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ParserState::Comment;
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
                        strspn!( comment_text, b'a'..=b'z'|b'A'..=b'Z'|b':'|b'_', 0 );

                    if 0 < pi_target_length {
                        pi_target_length += strspn!( comment_text, b'a'..=b'z'|b'A'..=b'Z'|b':'|b'_'|b'-'|b'.', pi_target_length );

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
                    self.parser_state = ParserState::IncompleteInput;
                    return false;
                }

                let closer_at = strpos_byte(&self.html_bytes, b'>', at + 2);
                if closer_at.is_none() {
                    self.parser_state = ParserState::IncompleteInput;
                    return false;
                }
                let closer_at = closer_at.unwrap();

                self.parser_state = ParserState::FunkyComment;
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
        self.parser_state = ParserState::TextNode;
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
            self.parser_state = ParserState::IncompleteInput;
            return false;
        }

        /*
         * Treat the equal sign as a part of the attribute
         * name if it is the first encountered byte.
         *
         * @see https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
         */
        let starts_with_equal = self.html_bytes[self.bytes_already_parsed] == b'=';
        let start_shift = if starts_with_equal { 1 } else { 0 };
        let name_length = start_shift
            + strcspn!(
                self.html_bytes,
                b'=' | b'/' | b'>' | b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                self.bytes_already_parsed + start_shift
            );

        // No attribute, just tag closer.
        if 0 == name_length || self.bytes_already_parsed + name_length >= doc_length {
            return false;
        }

        let attribute_start = self.bytes_already_parsed;
        self.bytes_already_parsed += name_length;
        if self.bytes_already_parsed >= doc_length {
            self.parser_state = ParserState::IncompleteInput;
            return false;
        }

        self.skip_whitespace();
        if self.bytes_already_parsed >= doc_length {
            self.parser_state = ParserState::IncompleteInput;
            return false;
        }

        let has_value = b'=' == self.html_bytes[self.bytes_already_parsed];
        let (value_start, value_length, attribute_end) = if has_value {
            self.bytes_already_parsed += 1;
            self.skip_whitespace();
            if self.bytes_already_parsed >= doc_length {
                self.parser_state = ParserState::IncompleteInput;
                return false;
            }

            match self.html_bytes[self.bytes_already_parsed] {
                quote @ (b'\'' | b'"') => {
                    let value_start = self.bytes_already_parsed + 1;
                    let end_quote_at = strpos_byte(&self.html_bytes, quote, value_start);
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
            self.parser_state = ParserState::IncompleteInput;
            return false;
        }

        if self.is_closing_tag.unwrap() {
            return true;
        }

        self.attributes.push(AttributeToken {
            name_length,
            value_starts_at: value_start,
            value_length,
            start: attribute_start,
            length: attribute_end - attribute_start,
            is_true: !has_value,
        });

        true
    }

    /// Returns the uppercase name of the matched tag.
    ///
    /// Example:
    ///
    ///     $p = new WP_HTML_Tag_Processor( '<div class="test">Test</div>' );
    ///     $p->next_tag() === true;
    ///     $p->get_tag() === 'DIV';
    ///
    ///     $p->next_tag() === false;
    ///     $p->get_tag() === null;
    ///
    /// @return string|null Name of currently matched tag in input HTML, or `null` if none found.
    pub fn get_tag(&self) -> Option<TagName> {
        if let (Some(at), Some(length)) = (self.tag_name_starts_at, self.tag_name_length) {
            let tag_name = substr(&self.html_bytes, at, length);

            if self.parser_state == ParserState::Comment
                && self.get_comment_type() == Some(&CommentType::PiNodeLookalike)
            {
                Some(TagName::Arbitrary(tag_name.into()))
            } else {
                Some((tag_name, &self.parsing_namespace).into())
            }
        } else {
            None
        }
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
        if self.parser_state != ParserState::MatchedTag {
            return None;
        }
        Some(qualified_attribute_name(
            attribute_name,
            self.get_namespace(),
        ))
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
    pub fn get_token_type(&self) -> Option<&TokenType> {
        match self.parser_state {
            ParserState::MatchedTag => Some(&TokenType::Tag),
            ParserState::Doctype => Some(&TokenType::Doctype),
            ParserState::TextNode => Some(&TokenType::Text),
            ParserState::CDATANode => Some(&TokenType::CdataSection),
            ParserState::Comment => Some(&TokenType::Comment),
            ParserState::PresumptuousTag => Some(&TokenType::PresumptuousTag),
            ParserState::FunkyComment => Some(&TokenType::FunkyComment),

            ParserState::Ready | ParserState::Complete | ParserState::IncompleteInput => None,
        }
    }

    pub fn get_token_name(&self) -> Option<NodeName> {
        match self.parser_state {
            ParserState::MatchedTag => Some(NodeName::Tag(self.get_tag().unwrap())),
            ParserState::Doctype => Some(NodeName::Token(TokenType::Doctype)),
            _ => self.get_token_type().map(|t| NodeName::Token(t.clone())),
        }
    }

    /// Skips contents of script tags.
    ///
    /// @return bool Whether the script tag was closed before the end of the document.
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
                if self.bytes_already_parsed >= doc_length {
                    return false;
                }

                while self.parse_next_attribute() {}

                if self.bytes_already_parsed >= doc_length {
                    self.parser_state = ParserState::IncompleteInput;
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

    /// Skips contents of RCDATA elements, namely title and textarea tags.
    ///
    /// @see https://html.spec.whatwg.org/multipage/parsing.html#rcdata-state
    ///
    /// @param string $tag_name The uppercase tag name which will close the RCDATA region.
    /// @return bool Whether an end to the RCDATA region was found before the end of the document.
    fn skip_rcdata(&mut self, tag_name: &TagName) -> bool {
        let doc_length = self.html_bytes.len();

        let mut at = self.bytes_already_parsed;

        let match_end_tag: &[u8] = match tag_name {
            TagName::IFRAME => b"</IFRAME",
            TagName::NOEMBED => b"</NOEMBED",
            TagName::NOFRAMES => b"</NOFRAMES",
            TagName::STYLE => b"</STYLE",
            TagName::TEXTAREA => b"</TEXTAREA",
            TagName::TITLE => b"</TITLE",
            TagName::XMP => b"</XMP",
            _ => unreachable!("skipping RCDATA at unexpected tag."),
        };

        while at + match_end_tag.len() + 1 < doc_length {
            at = if let Some(end_candidate_pos) = stripos(&self.html_bytes, match_end_tag, at) {
                self.tag_name_starts_at = Some(end_candidate_pos);
                end_candidate_pos + match_end_tag.len()
            } else {
                return false;
            };

            self.bytes_already_parsed = at;
            if at >= doc_length {
                return false;
            }

            /*
             * Ensure that the tag name terminates to avoid matching on
             * substrings of a longer tag name. For example, the sequence
             * "</textarearug" should not match for "</textarea" even
             * though "textarea" is found within the text.
             */
            if !matches!(
                self.html_bytes[at],
                b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>'
            ) {
                continue;
            }

            while self.parse_next_attribute() {}

            at = self.bytes_already_parsed;
            if at >= self.html_bytes.len() {
                return false;
            }

            if self.html_bytes[at] == b'>' {
                self.bytes_already_parsed = at + 1;
                return true;
            }

            if at + 1 >= self.html_bytes.len() {
                return false;
            }

            if &self.html_bytes[at..at + 2] == b"/>" {
                self.bytes_already_parsed = at + 2;
                return true;
            }
        }

        false
    }

    /// Skips contents of generic rawtext elements.
    ///
    /// @see https://html.spec.whatwg.org/#generic-raw-text-element-parsing-algorithm
    ///
    /// @param string $tag_name The uppercase tag name which will close the RAWTEXT region.
    /// @return bool Whether an end to the RAWTEXT region was found before the end of the document.
    fn skip_rawtext(&mut self, tag_name: &TagName) -> bool {
        /*
         * These two functions distinguish themselves on whether character references are
         * decoded, and since functionality to read the inner markup isn't supported, it's
         * not necessary to implement these two functions separately.
         */
        self.skip_rcdata(tag_name)
    }

    /// Move the internal cursor past any immediate successive whitespace.
    fn skip_whitespace(&mut self) {
        self.bytes_already_parsed += strspn!(
            &self.html_bytes,
            b' ' | b'\t' | 0x0C | b'\r' | b'\n',
            self.bytes_already_parsed
        );
    }

    /// Indicates if the current tag token is a tag closer.
    ///
    /// # Example:
    ///
    ///     $p = new WP_HTML_Tag_Processor( '<div></div>' );
    ///     $p->next_tag( array( 'tag_name' => 'div', 'tag_closers' => 'visit' ) );
    ///     $p->is_tag_closer() === false;
    ///
    ///     $p->next_tag( array( 'tag_name' => 'div', 'tag_closers' => 'visit' ) );
    ///     $p->is_tag_closer() === true;
    ///
    pub fn is_tag_closer(&self) -> bool {
        self.parser_state == ParserState::MatchedTag
            && self.is_closing_tag.unwrap_or(false)
            && self.get_tag().map(|t| t != TagName::BR).unwrap_or(false)
    }

    /// Returns if a matched tag contains the given ASCII case-insensitive class name.
    ///
    /// @param string $wanted_class Look for this CSS class name, ASCII case-insensitive.
    /// @return bool|null Whether the matched tag contains the given class name, or null if not matched.
    pub fn has_class(&self, wanted_class: &str) -> Option<bool> {
        if self.parser_state != ParserState::MatchedTag {
            return None;
        }

        let case_insensitive = self.compat_mode == CompatMode::Quirks;

        let wanted_bytes = wanted_class.as_bytes();
        Some(self.class_list().any(|class_name| {
            if case_insensitive {
                class_name.eq_ignore_ascii_case(wanted_bytes)
            } else {
                class_name.as_ref() == wanted_bytes
            }
        }))
    }

    /// Adds a new class name to the currently matched tag.
    ///
    /// @param class_name The class name to add.
    /// @return bool Whether the class was set to be added.
    pub fn add_class(&mut self, class_name: &str) -> bool {
        todo!()
    }

    /// Removes a class name from the currently matched tag.
    ///
    /// @param class_name The class name to remove.
    /// @return bool Whether the class was set to be removed.
    pub fn remove_class(&mut self, class_name: &str) -> bool {
        todo!()
    }

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
        self.parser_state == ParserState::IncompleteInput
    }

    /// Generator for a foreach loop to step through each class name for the matched tag.
    ///
    /// This generator function is designed to be used inside a "foreach" loop.
    ///
    /// Example:
    ///
    ///     $p = new WP_HTML_Tag_Processor( "<div class='free &lt;egg&lt;\tlang-en'>" );
    ///     $p->next_tag();
    ///     foreach ( $p->class_list() as $class_name ) {
    ///         echo "{$class_name} ";
    ///     }
    ///     // Outputs: "free <egg> lang-en "
    pub fn class_list(&self) -> ClassList {
        if self.parser_state != ParserState::MatchedTag {
            return ClassList::empty();
        }

        let is_quirks = self.compat_mode == CompatMode::Quirks;

        if let Some(class_attribute) = self.get_attribute(b"class") {
            match class_attribute {
                AttributeValue::String(class_attribute) => ClassList::new(class_attribute),
                _ => ClassList::empty(),
            }
        } else {
            ClassList::empty()
        }
    }

    /// Removes an attribute from the currently matched tag.
    ///
    /// @param name The attribute name to remove.
    /// @return bool Whether the attribute was set to be removed.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
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
    /// @param string $name Identifies this particular bookmark.
    /// @return bool Whether the bookmark was successfully created.
    ///
    pub fn set_bookmark<'a, Mark: Into<BookmarkName>>(&mut self, name: Mark) -> Result<(), ()> {
        // It only makes sense to set a bookmark if the parser has paused on a concrete token.
        if matches!(
            self.parser_state,
            ParserState::Complete | ParserState::IncompleteInput
        ) {
            return Err(());
        }

        let mark: BookmarkName = name.into();
        let total_marks = self.internal_bookmarks.len() + self.bookmarks.len();
        let over_size = total_marks >= MAX_BOOKMARKS;
        match mark {
            BookmarkName::Internal(i) => {
                if !self.internal_bookmarks.contains_key(&i) && over_size {
                    return Err(());
                }
                let span = HtmlSpan::new(self.token_starts_at.unwrap(), self.token_length.unwrap());
                self.internal_bookmarks.insert(i, span);
            }
            BookmarkName::String(s) => {
                if !self.bookmarks.contains_key(s.as_ref()) && over_size {
                    return Err(());
                }

                let span = HtmlSpan::new(self.token_starts_at.unwrap(), self.token_length.unwrap());
                self.bookmarks.insert(s, span);
            }
        };

        Ok(())
    }

    /// Removes a bookmark that is no longer needed.
    ///
    /// Releasing a bookmark frees up the small
    /// performance overhead it requires.
    ///
    /// @param name Name of the bookmark to remove.
    /// @return bool Whether the bookmark already existed before removal.
    pub fn release_bookmark<'a, Mark: Into<BookmarkName>>(&mut self, name: Mark) -> bool {
        let name = name.into();
        match name {
            BookmarkName::Internal(i) => self.internal_bookmarks.remove(&i).is_some(),
            BookmarkName::String(s) => self.bookmarks.remove(s.as_ref()).is_some(),
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
    pub fn get_attribute_names_with_prefix(&self, prefix: &[u8]) -> Option<Vec<Box<[u8]>>> {
        if self.parser_state != ParserState::MatchedTag || self.is_closing_tag.unwrap_or(false) {
            return None;
        }

        Some(
            self.attributes
                .iter()
                .filter_map(
                    |AttributeToken {
                         start, name_length, ..
                     }| {
                        if *name_length < prefix.len() {
                            None
                        } else {
                            let name = &self.html_bytes[*start..start + prefix.len()];
                            if name.eq_ignore_ascii_case(prefix) {
                                let slice = self.html_bytes[*start..start + name_length]
                                    .to_ascii_lowercase();
                                Some(slice.into_boxed_slice())
                            } else {
                                None
                            }
                        }
                    },
                )
                .collect::<Vec<_>>(),
        )
    }

    /// Returns the namespace of the matched token.
    pub fn get_namespace(&self) -> &ParsingNamespace {
        &self.parsing_namespace
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
    /// Limitations:
    ///
    ///  - This function will not strip the leading newline appropriately
    ///    after seeking into a LISTING or PRE element. To ensure that the
    ///    newline is treated properly, seek to the LISTING or PRE opening
    ///    tag instead of to the first text node inside the element.
    ///
    /// @return string
    pub fn get_modifiable_text(&self) -> Box<[u8]> {
        // TODO: This isn't the same, PHP uses mixed numeric/associative array keys for lexical
        // updates and a named key "modifiable text".
        let has_enqueued_update = !self.lexical_updates.is_empty();

        if !has_enqueued_update
            && (self.text_starts_at.is_none() || self.text_length.is_none_or(|x| x == 0))
        {
            return Box::new([]);
        }

        if has_enqueued_update {
            unimplemented!("Lexical update handling is not yet implemented get_modifiable_text.");
        }

        let text_starts_at = if let Some(text_starts_at) = self.text_starts_at {
            text_starts_at
        } else {
            return Box::new([]);
        };
        let text_length = if let Some(text_length) = self.text_length {
            text_length
        } else {
            return Box::new([]);
        };

        let text = {
            let text = substr(&self.html_bytes, text_starts_at, text_length);

            /*
             * Pre-processing the input stream would normally happen before
             * any parsing is done, but deferring it means it's possible to
             * skip in most cases. When getting the modifiable text, however
             * it's important to apply the pre-processing steps, which is
             * normalizing newlines.
             *
             * @see https://html.spec.whatwg.org/#preprocessing-the-input-stream
             * @see https://infra.spec.whatwg.org/#normalize-newlines
             */

            let mut text_normalized: Vec<u8> = Vec::new();
            let mut chars = text.iter().peekable();
            while let Some(&c) = chars.next() {
                match c {
                    b'\r' => {
                        if chars.peek() == Some(&&b'\n') {
                            chars.next(); // consume the following '\n'
                            text_normalized.push(b'\n');
                        } else {
                            text_normalized.push(b'\n');
                        }
                    }
                    _ => text_normalized.push(c),
                }
            }
            text_normalized
        };

        // Comment data is not decoded.
        if matches!(
            self.parser_state,
            ParserState::CDATANode
                | ParserState::Comment
                | ParserState::Doctype
                | ParserState::FunkyComment
        ) {
            return text
                .iter()
                .flat_map(|&c| -> Vec<u8> {
                    if c == b'\0' {
                        "\u{FFFD}".as_bytes().into()
                    } else {
                        vec![c]
                    }
                })
                .collect::<Vec<u8>>()
                .into();
        }

        if let Some(tag_name) = self.get_tag() {
            if matches!(
                tag_name,
                // Script data is not decoded.
                TagName::SCRIPT
                    |

                // RAWTEXT data is not decoded.
                TagName::IFRAME
                    | TagName::NOEMBED
                    | TagName::NOFRAMES
                    | TagName::STYLE
                    | TagName::XMP
            ) {
                return text
                    .iter()
                    .flat_map(|&c| -> Vec<u8> {
                        if c == b'\0' {
                            "\u{FFFD}".as_bytes().into()
                        } else {
                            vec![c]
                        }
                    })
                    .collect::<Vec<u8>>()
                    .into();
            }
        }

        let mut text = entities::decode(&entities::HtmlContext::BodyText, &text).to_vec();

        /*
         * Skip the first line feed after LISTING, PRE, and TEXTAREA opening tags.
         *
         * Note that this first newline may come in the form of a character
         * reference, such as `&#x0a;`, and so it's important to perform
         * this transformation only after decoding the raw text content.
         */
        if b'\n' == text[0] {
            if let Some(token_name) = self.get_token_name() {
                if (self.skip_newline_at.is_some()
                    && self.skip_newline_at == self.token_starts_at
                    && token_name == NodeName::Token(TokenType::Text))
                    || token_name == NodeName::Tag(TagName::TEXTAREA)
                {
                    text.remove(0);
                }
            }
        }

        /*
         * Only in normative text nodes does the NULL byte (U+0000) get removed.
         * In all other contexts it's replaced by the replacement character (U+FFFD)
         * for security reasons (to avoid joining together strings that were safe
         * when separated, but not when joined).
         *
         * @todo Inside HTML integration points and MathML integration points, the
         *       text is processed according to the insertion mode, not according
         *       to the foreign content rules. This should strip the NULL bytes.
         */
        if self.parsing_namespace == ParsingNamespace::Html
            && self.get_token_type() == Some(&TokenType::Text)
        {
            text.into_iter()
                .filter(|&c| c != b'\0')
                .collect::<Vec<u8>>()
                .into()
        } else {
            text.into_iter()
                .flat_map(|c| -> Vec<u8> {
                    if c == b'\0' {
                        "\u{FFFD}".as_bytes().into()
                    } else {
                        vec![c]
                    }
                })
                .collect::<Vec<u8>>()
                .into()
        }
    }

    /// Sets the modifiable text for the matched token, if matched.
    ///
    /// Modifiable text is text content that may be read and changed without
    /// changing the HTML structure of the document around it. This includes
    /// the contents of `#text` nodes in the HTML as well as the inner
    /// contents of HTML comments, Processing Instructions, and others, even
    /// though these nodes aren't part of a parsed DOM tree. They also contain
    /// the contents of SCRIPT and STYLE tags, of TEXTAREA tags, and of any
    /// other section in an HTML document which cannot contain HTML markup (DATA).
    ///
    /// Not all modifiable text may be set by this method, and not all content
    /// may be set as modifiable text. In the case that this fails it will return
    /// `false` indicating as much. For instance, it will not allow inserting the
    /// string `</script` into a SCRIPT element, because the rules for escaping
    /// that safely are complicated. Similarly, it will not allow setting content
    /// into a comment which would prematurely terminate the comment.
    ///
    /// Example:
    ///
    ///     // Add a preface to all STYLE contents.
    ///     while ( $processor->next_tag( 'STYLE' ) ) {
    ///         $style = $processor->get_modifiable_text();
    ///         $processor->set_modifiable_text( "// Made with love on the World Wide Web\n{$style}" );
    ///     }
    ///
    ///     // Replace smiley text with Emoji smilies.
    ///     while ( $processor->next_token() ) {
    ///         if ( '#text' !== $processor->get_token_name() ) {
    ///             continue;
    ///         }
    ///
    ///         $chunk = $processor->get_modifiable_text();
    ///         if ( ! str_contains( $chunk, ':)' ) ) {
    ///             continue;
    ///         }
    ///
    ///         $processor->set_modifiable_text( str_replace( ':)', '🙂', $chunk ) );
    ///     }
    ///
    /// @param string $plaintext_content New text content to represent in the matched token.
    ///
    /// @return bool Whether the text was able to update.
    pub fn set_modifiable_text(&mut self, updated_text: &str) -> bool {
        unimplemented!("set_modifiable_text is not yet implemented.")
    }

    /// Checks whether a bookmark with the given name exists.
    ///
    /// @param bookmark_name Name to identify a bookmark that potentially exists.
    /// @return Whether that bookmark exists.
    pub fn has_bookmark(&self, bookmark_name: &str) -> bool {
        self.bookmarks.contains_key(bookmark_name)
    }

    /// Move the internal cursor in the Tag Processor to a given bookmark's location.
    ///
    /// In order to prevent accidental infinite loops, there's a
    /// maximum limit on the number of times seek() can be called.
    ///
    /// @param bookmark_name Jump to the place in the document identified by this bookmark name.
    /// @return Whether the internal cursor was successfully moved to the bookmark's location.
    pub fn seek(&mut self, bookmark_name: &str) -> bool {
        todo!()
    }

    pub fn get_comment_type(&self) -> Option<&CommentType> {
        if self.parser_state != ParserState::Comment {
            None
        } else {
            self.comment_type.as_ref()
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
        if self.parser_state == ParserState::FunkyComment {
            return Some(self.get_modifiable_text());
        }

        let comment_type = self.get_comment_type()?;

        Some(match comment_type {
            CommentType::HtmlComment | CommentType::AbruptlyClosedComment => {
                self.get_modifiable_text()
            }

            CommentType::CdataLookalike => {
                let mut text = b"[CDATA[".to_vec();
                text.extend(&self.get_modifiable_text());
                text.extend(b"]]");
                text.into()
            }

            CommentType::PiNodeLookalike => {
                let pi_target = self.get_tag().unwrap();
                let pi_target: Box<[u8]> = pi_target.into();
                let mut text = b"?".to_vec();
                text.extend(pi_target);
                text.extend(&self.get_modifiable_text());
                text.push(b'?');
                text.into()
            }

            /*
             * This represents "bogus comments state" from HTML tokenization.
             * This can be entered by `<?` or `<!`, where `?` is included in
             * the comment text but `!` is not.
             */
            CommentType::InvalidHtml => {
                let mut text = Vec::new();
                let preceding_byte = self.html_bytes[self.text_starts_at.unwrap() - 1];
                if preceding_byte == b'?' {
                    text.push(b'?');
                }
                text.extend(&self.get_modifiable_text());
                text.into()
            }
        })
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) -> bool {
        todo!()
    }

    pub fn get_attribute(&self, name: &[u8]) -> Option<AttributeValue> {
        if self.parser_state != ParserState::MatchedTag {
            return None;
        }

        if !self.lexical_updates.is_empty() {
            todo!("Get attribute lexical update handling.");
        }

        Some(
            if let Some(attr_token) = self.attributes.iter().find(|&token| {
                let attr_name = &self.html_bytes[token.start..token.start + token.name_length];
                attr_name.eq_ignore_ascii_case(name)
            }) {
                if attr_token.is_true {
                    AttributeValue::BooleanTrue
                } else {
                    let raw_value = &self.html_bytes[attr_token.value_starts_at
                        ..attr_token.value_starts_at + attr_token.value_length];
                    let decoded = entities::decode(&entities::HtmlContext::Attribute, raw_value);
                    AttributeValue::String(Box::from(decoded))
                }
            } else {
                AttributeValue::BooleanFalse
            },
        )
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
        if self.parser_state != ParserState::MatchedTag {
            return false;
        }

        /*
         * The self-closing flag is the solidus at the _end_ of the tag, not the beginning.
         *
         * Example:
         *
         *     <figure />
         *             ^ this appears one character before the end of the closing ">".
         */
        b'/' == self.html_bytes[self.token_starts_at.unwrap() + self.token_length.unwrap() - 2]
    }

    /**
     * Subdivides a matched text node, splitting NULL byte sequences and decoded whitespace as
     * distinct nodes prefixes.
     *
     * Note that once anything that's neither a NULL byte nor decoded whitespace is
     * encountered, then the remainder of the text node is left intact as generic text.
     *
     *  - The HTML Processor uses this to apply distinct rules for different kinds of text.
     *  - Inter-element whitespace can be detected and skipped with this method.
     *
     * Text nodes aren't eagerly subdivided because there's no need to split them unless
     * decisions are being made on NULL byte sequences or whitespace-only text.
     *
     * Example:
     *
     *     $processor = new WP_HTML_Tag_Processor( "\x00Apples & Oranges" );
     *     true  === $processor->next_token();                   // Text is "Apples & Oranges".
     *     true  === $processor->subdivide_text_appropriately(); // Text is "".
     *     true  === $processor->next_token();                   // Text is "Apples & Oranges".
     *     false === $processor->subdivide_text_appropriately();
     *
     *     $processor = new WP_HTML_Tag_Processor( "&#x13; \r\n\tMore" );
     *     true  === $processor->next_token();                   // Text is "␤ ␤␉More".
     *     true  === $processor->subdivide_text_appropriately(); // Text is "␤ ␤␉".
     *     true  === $processor->next_token();                   // Text is "More".
     *     false === $processor->subdivide_text_appropriately();
     *
     * @return bool Whether the text node was subdivided.
     */
    pub fn subdivide_text_appropriately(&mut self) -> bool {
        if self.parser_state != ParserState::TextNode {
            return false;
        }

        self.text_node_classification = TextNodeClassification::Generic;

        /*
         * NULL bytes are treated categorically different than numeric character
         * references whose number is zero. `&#x00;` is not the same as `"\x00"`.
         */
        let leading_nulls = strspn!(
            &self.html_bytes,
            b'\x00',
            self.text_starts_at.unwrap(),
            self.text_length.unwrap()
        );
        if leading_nulls > 0 {
            self.token_length = Some(leading_nulls);
            self.text_length = Some(leading_nulls);
            self.bytes_already_parsed = self.token_starts_at.unwrap() + leading_nulls;
            self.text_node_classification = TextNodeClassification::NullSequence;
            return true;
        }

        let mut at = self.text_starts_at.unwrap();
        let end = self.text_starts_at.unwrap() + self.text_length.unwrap();
        while at < end {
            let skipped = strspn!(
                self.html_bytes,
                b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                at,
                end - at
            );
            at += skipped;

            if at < end && b'&' == self.html_bytes[at] {
                if let Some((decoded, matched_byte_length)) = entities::decode_html_ref(
                    &entities::HtmlContext::BodyText,
                    &self.html_bytes,
                    at,
                ) {
                    if matches!(decoded[0], b' ' | b'\t' | 0x0c | b'\r' | b'\n',) {
                        at += matched_byte_length;
                        continue;
                    }
                }
            }

            break;
        }

        if at > self.text_starts_at.unwrap() {
            let new_length = at - self.text_starts_at.unwrap();
            self.text_length = Some(new_length);
            self.token_length = Some(new_length);
            self.bytes_already_parsed = at;
            self.text_node_classification = TextNodeClassification::Whitespace;
            return true;
        }

        false
    }

    pub fn change_parsing_namespace(&mut self, namespace: ParsingNamespace) -> bool {
        self.parsing_namespace = namespace;
        true
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
        if self.parser_state != ParserState::Doctype {
            None
        } else {
            HtmlDoctypeInfo::from_doctype_token(
                &self.html_bytes[self.token_starts_at.unwrap()
                    ..self.token_starts_at.unwrap() + self.token_length.unwrap()],
            )
        }
    }

    /// Needed to expose PHP protected proptery.
    pub fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

impl Default for TagProcessor {
    fn default() -> Self {
        Self {
            attributes: vec![],
            bytes_already_parsed: 0,
            comment_type: None,
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
            compat_mode: Default::default(),
            bookmarks: HashMap::new(),
            internal_bookmarks: FxHashMap::default(),
        }
    }
}

#[derive(Default, PartialEq, Debug)]
pub enum ParserState {
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextNodeClassification {
    Generic,
    NullSequence,
    Whitespace,
}

#[derive(Clone, PartialEq)]
pub enum CommentType {
    /**
     * Indicates that a comment was created when encountering abruptly-closed HTML comment.
     *
     * Example:
     *
     *     <!-->
     *     <!--->
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
     */
    CdataLookalike,

    /**
     * Indicates that a comment was created when encountering
     * normative HTML comment syntax.
     *
     * Example:
     *
     *     <!-- this is a comment -->
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
     */
    InvalidHtml,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Tag,
    Text,
    CdataSection,
    Comment,
    Doctype,
    PresumptuousTag,
    FunkyComment,
}

impl From<&TokenType> for String {
    fn from(val: &TokenType) -> Self {
        let s: &str = val.into();
        s.into()
    }
}
impl From<TokenType> for String {
    fn from(val: TokenType) -> Self {
        let s: &str = (&val).into();
        s.into()
    }
}

impl From<&TokenType> for &str {
    fn from(val: &TokenType) -> Self {
        match val {
            TokenType::Tag => "#tag",
            TokenType::Text => "#text",
            TokenType::CdataSection => "#cdata-section",
            TokenType::Comment => "#comment",
            TokenType::Doctype => "#doctype",
            TokenType::PresumptuousTag => "#presumptuous-tag",
            TokenType::FunkyComment => "#funky-comment",
        }
    }
}

#[derive(Debug, Clone)]
struct AttributeToken {
    // The byte length of the name.
    pub name_length: usize,

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_base_next_token() {
        let mut processor = TagProcessor::new(b"<p>Hello world!</p>");
        assert!(processor.base_class_next_token());
        assert_eq!(processor.get_token_type().unwrap(), &TokenType::Tag);
        assert_eq!(processor.get_token_name().unwrap(), TagName::P.into());
        assert_eq!(processor.get_tag().unwrap(), TagName::P);
        assert!(processor.base_class_next_token());
        assert_eq!(processor.get_token_type().unwrap(), &TokenType::Text);
        assert_eq!(processor.get_token_name().unwrap(), TokenType::Text.into());
        assert!(processor.base_class_next_token());
        assert_eq!(processor.get_token_type().unwrap(), &TokenType::Tag);
        assert_eq!(processor.get_token_name().unwrap(), TagName::P.into());
        assert!(processor.is_tag_closer());
    }
}
#[derive(PartialEq, Clone, Debug)]
pub enum NodeName {
    Tag(TagName),
    Token(TokenType),
}
impl NodeName {
    pub fn tag(&self) -> Option<&TagName> {
        match self {
            NodeName::Tag(tag_name) => Some(tag_name),
            _ => None,
        }
    }
}
impl From<TagName> for NodeName {
    fn from(val: TagName) -> Self {
        NodeName::Tag(val)
    }
}
impl From<TokenType> for NodeName {
    fn from(val: TokenType) -> Self {
        NodeName::Token(val)
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub enum AttributeValue {
    #[default]
    BooleanFalse,
    BooleanTrue,
    String(Box<[u8]>),
}

pub struct ClassList {
    attribute_value: Box<[u8]>,
    seen: BTreeSet<Box<[u8]>>,
    at: usize,
}

impl ClassList {
    pub fn new(attribute_value: Box<[u8]>) -> Self {
        let mut s = Self {
            attribute_value,
            seen: BTreeSet::new(),
            at: 0,
        };

        // Start by skipping whitespace.
        s.at = strspn!(s.attribute_value, b' ' | b'\t' | 0x0c | b'\r' | b'\n', s.at);

        s
    }

    pub fn empty() -> Self {
        Self {
            attribute_value: Box::new([]),
            at: 0,
            seen: BTreeSet::new(),
        }
    }
}

impl Iterator for ClassList {
    type Item = Box<[u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.attribute_value.len() {
            return None;
        }

        // Find the byte length until the next boundary.
        let len = strcspn!(
            self.attribute_value,
            b' ' | b'\t' | 0x0c | b'\r' | b'\n',
            self.at
        );

        let value = &self.attribute_value[self.at..self.at + len];

        // Move past trailing whitespace.
        self.at += len
            + strspn!(
                self.attribute_value,
                b' ' | b'\t' | 0x0c | b'\r' | b'\n',
                self.at + len
            );

        let value: Vec<u8> = value
            .iter()
            .flat_map(|&byte| {
                if byte == 0x00 {
                    b"\xEF\xBF\xBD".to_vec()
                } else {
                    vec![byte]
                }
            })
            .collect();

        let value = value.into_boxed_slice();

        if self.seen.contains(&value) {
            self.next()
        } else {
            self.seen.insert(value.clone());
            Some(value)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum BookmarkName {
    Internal(u32),
    String(Box<str>),
}

impl From<&str> for BookmarkName {
    fn from(other: &str) -> Self {
        BookmarkName::String(other.into())
    }
}
