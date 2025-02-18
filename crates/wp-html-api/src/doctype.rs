use crate::compat_mode::CompatMode;

/**
 * HTML API: WP_HTML_Doctype_Info class
 *
 * @package WordPress
 * @subpackage HTML-API
 * @since 6.7.0
 */

/**
 * Core class used by the HTML API to represent a DOCTYPE declaration.
 *
 * This class parses DOCTYPE tokens for the full parser in the HTML Processor.
 * Most code interacting with HTML won't need to parse DOCTYPE declarations;
 * the HTML Processor is one exception. Consult the HTML Processor for proper
 * parsing of an HTML document.
 *
 * A DOCTYPE declaration may indicate its document compatibility mode, which impacts
 * the structure of the following HTML as well as the behavior of CSS class selectors.
 * There are three possible modes:
 *
 *  - "no-quirks" and "limited-quirks" modes (also called "standards mode").
 *  - "quirks" mode.
 *
 * These modes mostly determine whether CSS class name selectors match values in the
 * HTML `class` attribute in an ASCII-case-insensitive way (quirks mode), or whether
 * they match only when byte-for-byte identical (no-quirks mode).
 *
 * All HTML documents should start with the standard HTML5 DOCTYPE: `<!DOCTYPE html>`.
 *
 * > DOCTYPEs are required for legacy reasons. When omitted, browsers tend to use a different
 * > rendering mode that is incompatible with some specifications. Including the DOCTYPE in a
 * > document ensures that the browser makes a best-effort attempt at following the
 * > relevant specifications.
 *
 * @see https://html.spec.whatwg.org/#the-doctype
 *
 * DOCTYPE declarations comprise four properties: a name, public identifier, system identifier,
 * and an indication of which document compatability mode they would imply if an HTML parser
 * hadn't already determined it from other information.
 *
 * @see https://html.spec.whatwg.org/#the-initial-insertion-mode
 *
 * Historically, the DOCTYPE declaration was used in SGML documents to instruct a parser how
 * to interpret the various tags and entities within a document. Its role in HTML diverged
 * from how it was used in SGML and no meaning should be back-read into HTML based on how it
 * is used in SGML, XML, or XHTML documents.
 *
 * @see https://www.iso.org/standard/16387.html
 *
 * @since 6.7.0
 *
 * @see WP_HTML_Processor
 */
pub struct HtmlDoctypeInfo {
    /**
     * Name of the DOCTYPE: should be "html" for HTML documents.
     *
     * This value should be considered "read only" and not modified.
     *
     * Historically the DOCTYPE name indicates name of the document's root element.
     *
     *     <!DOCTYPE html>
     *               ╰──┴── name is "html".
     *
     * @see https://html.spec.whatwg.org/#tokenization
     *
     * @since 6.7.0
     *
     * @var string|null
     */
    pub name: Option<Box<[u8]>>,

    /**
     * Public identifier of the DOCTYPE.
     *
     * This value should be considered "read only" and not modified.
     *
     * The public identifier is optional and should not appear in HTML documents.
     * A `null` value indicates that no public identifier was present in the DOCTYPE.
     *
     * Historically the presence of the public identifier indicated that a document
     * was meant to be shared between computer systems and the value indicated to a
     * knowledgeable parser how to find the relevant document type definition (DTD).
     *
     *     <!DOCTYPE html PUBLIC "public id goes here in quotes">
     *               │  │         ╰─── public identifier ─────╯
     *               ╰──┴── name is "html".
     *
     * @see https://html.spec.whatwg.org/#tokenization
     *
     * @since 6.7.0
     *
     * @var string|null
     */
    pub public_identifier: Option<Box<[u8]>>,

    /**
     * System identifier of the DOCTYPE.
     *
     * This value should be considered "read only" and not modified.
     *
     * The system identifier is optional and should not appear in HTML documents.
     * A `null` value indicates that no system identifier was present in the DOCTYPE.
     *
     * Historically the system identifier specified where a relevant document type
     * declaration for the given document is stored and may be retrieved.
     *
     *     <!DOCTYPE html SYSTEM "system id goes here in quotes">
     *               │  │         ╰──── system identifier ────╯
     *               ╰──┴── name is "html".
     *
     * If a public identifier were provided it would indicate to a knowledgeable
     * parser how to interpret the system identifier.
     *
     *     <!DOCTYPE html PUBLIC "public id goes here in quotes" "system id goes here in quotes">
     *               │  │         ╰─── public identifier ─────╯   ╰──── system identifier ────╯
     *               ╰──┴── name is "html".
     *
     * @see https://html.spec.whatwg.org/#tokenization
     *
     * @since 6.7.0
     *
     * @var string|null
     */
    pub system_identifier: Option<Box<[u8]>>,

    /**
     * Which document compatability mode this DOCTYPE declaration indicates.
     *
     * This value should be considered "read only" and not modified.
     *
     * When an HTML parser has not already set the document compatability mode,
     * (e.g. "quirks" or "no-quirks" mode), it will infer if from the properties
     * of the appropriate DOCTYPE declaration, if one exists. The DOCTYPE can
     * indicate one of three possible document compatability modes:
     *
     *  - "no-quirks" and "limited-quirks" modes (also called "standards" mode).
     *  - "quirks" mode (also called `CSS1Compat` mode).
     *
     * An appropriate DOCTYPE is one encountered in the "initial" insertion mode,
     * before the HTML element has been opened and before finding any other
     * DOCTYPE declaration tokens.
     *
     * @see https://html.spec.whatwg.org/#the-initial-insertion-mode
     *
     * @since 6.7.0
     *
     * @var string One of "no-quirks", "limited-quirks", or "quirks".
     */
    pub indicated_compatability_mode: CompatMode,
}

impl HtmlDoctypeInfo {
    /**
     * Constructor.
     *
     * This class should not be instantiated directly.
     * Use the static {@see self::from_doctype_token} method instead.
     *
     * The arguments to this constructor correspond to the "DOCTYPE token"
     * as defined in the HTML specification.
     *
     * > DOCTYPE tokens have a name, a public identifier, a system identifier,
     * > and a force-quirks flag. When a DOCTYPE token is created, its name, public identifier,
     * > and system identifier must be marked as missing (which is a distinct state from the
     * > empty string), and the force-quirks flag must be set to off (its other state is on).
     *
     * @see https://html.spec.whatwg.org/multipage/parsing.html#tokenization
     *
     * @since 6.7.0
     *
     * @param string|null $name              Name of the DOCTYPE.
     * @param string|null $public_identifier Public identifier of the DOCTYPE.
     * @param string|null $system_identifier System identifier of the DOCTYPE.
     * @param bool        $force_quirks_flag Whether the force-quirks flag is set for the token.
     */
    fn new(
        name: Option<Box<[u8]>>,
        public_identifier: Option<Box<[u8]>>,
        system_identifier: Option<Box<[u8]>>,
        force_quirks_flag: bool,
    ) -> Self {
        /*
         * > If the DOCTYPE token matches one of the conditions in the following list,
         * > then set the Document to quirks mode:
         */

        /*
         * > The force-quirks flag is set to on.
         */
        if force_quirks_flag {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * Normative documents will contain the literal `<!DOCTYPE html>` with no
         * public or system identifiers; short-circuit to avoid extra parsing.
         */
        if name
            .as_ref()
            .map(|n| n.as_ref() == b"html")
            .unwrap_or(false)
            && public_identifier.is_none()
            && system_identifier.is_none()
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::NoQuirks,
            };
        }

        /*
         * > The name is not "html".
         *
         * The tokenizer must report the name in lower case even if provided in
         * the document in upper case; thus no conversion is required here.
         */
        if !name
            .as_ref()
            .map(|n| n.as_ref() == b"html")
            .unwrap_or(false)
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * Set up some variables to handle the rest of the conditions.
         *
         * > set...the public identifier...to...the empty string if the public identifier was missing.
         * > set...the system identifier...to...the empty string if the system identifier was missing.
         * >
         * > The system identifier and public identifier strings must be compared...
         * > in an ASCII case-insensitive manner.
         * >
         * > A system identifier whose value is the empty string is not considered missing
         * > for the purposes of the conditions above.
         */
        let system_identifier_is_missing = system_identifier.is_none();
        let public_identifier_cleaned = match public_identifier {
            Some(ref s) => s.to_ascii_lowercase(),
            None => vec![],
        };
        let system_identifier_cleaned = match system_identifier {
            Some(ref s) => s.to_ascii_lowercase(),
            None => vec![],
        };

        /*
         * > The public identifier is set to…
         */
        if *b"-//w3o//dtd w3 html strict 3.0//en//" == *public_identifier_cleaned
            || *b"-/w3c/dtd html 4.0 transitional/en" == *public_identifier_cleaned
            || *b"html" == *public_identifier_cleaned
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * > The system identifier is set to…
         */
        if *b"http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd"
            == *system_identifier_cleaned
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * All of the following conditions depend on matching the public identifier.
         * If the public identifier is empty, none of the following conditions will match.
         */
        if public_identifier_cleaned.is_empty() {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * > The public identifier starts with…
         *
         * @todo Optimize this matching. It shouldn't be a large overall performance issue,
         *       however, as only a single DOCTYPE declaration token should ever be parsed,
         *       and normative documents will have exited before reaching this condition.
         */
        if public_identifier_cleaned.starts_with(b"+//silmaril//dtd html pro v0r11 19970101//")
            || public_identifier_cleaned.starts_with(b"-//as//dtd html 3.0 aswedit + extensions//")
            || public_identifier_cleaned
                .starts_with(b"-//advasoft ltd//dtd html 3.0 aswedit + extensions//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0 level 1//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0 level 2//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0 strict level 1//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0 strict level 2//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0 strict//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.0//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 2.1e//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 3.0//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 3.2 final//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 3.2//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html 3//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html level 0//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html level 1//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html level 2//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html level 3//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html strict level 0//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html strict level 1//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html strict level 2//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html strict level 3//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html strict//")
            || public_identifier_cleaned.starts_with(b"-//ietf//dtd html//")
            || public_identifier_cleaned.starts_with(b"-//metrius//dtd metrius presentational//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 2.0 html strict//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 2.0 html//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 2.0 tables//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 3.0 html strict//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 3.0 html//")
            || public_identifier_cleaned
                .starts_with(b"-//microsoft//dtd internet explorer 3.0 tables//")
            || public_identifier_cleaned.starts_with(b"-//netscape comm. corp.//dtd html//")
            || public_identifier_cleaned.starts_with(b"-//netscape comm. corp.//dtd strict html//")
            || public_identifier_cleaned.starts_with(b"-//o'reilly and associates//dtd html 2.0//")
            || public_identifier_cleaned
                .starts_with(b"-//o'reilly and associates//dtd html extended 1.0//")
            || public_identifier_cleaned
                .starts_with(b"-//o'reilly and associates//dtd html extended relaxed 1.0//")
            || public_identifier_cleaned.starts_with(b"-//sq//dtd html 2.0 hotmetal + extensions//")
            || public_identifier_cleaned.starts_with(
                b"-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
            )
            || public_identifier_cleaned.starts_with(
                b"-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
            )
            || public_identifier_cleaned.starts_with(b"-//spyglass//dtd html 2.0 extended//")
            || public_identifier_cleaned
                .starts_with(b"-//sun microsystems corp.//dtd hotjava html//")
            || public_identifier_cleaned
                .starts_with(b"-//sun microsystems corp.//dtd hotjava strict html//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 3 1995-03-24//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 3.2 draft//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 3.2 final//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 3.2//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 3.2s draft//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.0 frameset//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.0 transitional//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html experimental 19960712//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd html experimental 970421//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd w3 html//")
            || public_identifier_cleaned.starts_with(b"-//w3o//dtd w3 html 3.0//")
            || public_identifier_cleaned.starts_with(b"-//webtechs//dtd mozilla html 2.0//")
            || public_identifier_cleaned.starts_with(b"-//webtechs//dtd mozilla html//")
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * > The system identifier is missing and the public identifier starts with…
         */
        if system_identifier_is_missing
            && (public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.01 frameset//")
                || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.01 transitional//"))
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::Quirks,
            };
        }

        /*
         * > Otherwise, if the DOCTYPE token matches one of the conditions in
         * > the following list, then set the Document to limited-quirks mode.
         */

        /*
         * > The public identifier starts with…
         */
        if public_identifier_cleaned.starts_with(b"-//w3c//dtd xhtml 1.0 frameset//")
            || public_identifier_cleaned.starts_with(b"-//w3c//dtd xhtml 1.0 transitional//")
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::LimitedQuirks,
            };
        }

        /*
         * > The system identifier is not missing and the public identifier starts with…
         */
        if !system_identifier_is_missing
            && (public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.01 frameset//")
                || public_identifier_cleaned.starts_with(b"-//w3c//dtd html 4.01 transitional//"))
        {
            return Self {
                name,
                public_identifier,
                system_identifier,
                indicated_compatability_mode: CompatMode::LimitedQuirks,
            };
        }

        Self {
            name,
            public_identifier,
            system_identifier,
            indicated_compatability_mode: CompatMode::NoQuirks,
        }
    }

    /**
     * Creates a WP_HTML_Doctype_Info instance by parsing a raw DOCTYPE declaration token.
     *
     * Use this method to parse a DOCTYPE declaration token and get access to its properties
     * via the returned WP_HTML_Doctype_Info class instance. The provided input must parse
     * properly as a DOCTYPE declaration, though it must not represent a valid DOCTYPE.
     *
     * Example:
     *
     *     // Normative HTML DOCTYPE declaration.
     *     $doctype = WP_HTML_Doctype_Info::from_doctype_token( '<!DOCTYPE html>' );
     *     'no-quirks' === $doctype->indicated_compatability_mode;
     *
     *     // A nonsensical DOCTYPE is still valid, and will indicate "quirks" mode.
     *     $doctype = WP_HTML_Doctype_Info::from_doctype_token( '<!doctypeJSON SILLY "nonsense\'>' );
     *     'quirks' === $doctype->indicated_compatability_mode;
     *
     *     // Textual quirks present in raw HTML are handled appropriately.
     *     $doctype = WP_HTML_Doctype_Info::from_doctype_token( "<!DOCTYPE\nhtml\n>" );
     *     'no-quirks' === $doctype->indicated_compatability_mode;
     *
     *     // Anything other than a proper DOCTYPE declaration token fails to parse.
     *     null === WP_HTML_Doctype_Info::from_doctype_token( ' <!DOCTYPE>' );
     *     null === WP_HTML_Doctype_Info::from_doctype_token( '<!DOCTYPE ><p>' );
     *     null === WP_HTML_Doctype_Info::from_doctype_token( '<!TYPEDOC>' );
     *     null === WP_HTML_Doctype_Info::from_doctype_token( 'html' );
     *     null === WP_HTML_Doctype_Info::from_doctype_token( '<?xml version="1.0" encoding="UTF-8" ?>' );
     *
     * @since 6.7.0
     *
     * @param string $doctype_html The complete raw DOCTYPE HTML string, e.g. `<!DOCTYPE html>`.
     *
     * @return WP_HTML_Doctype_Info|null A WP_HTML_Doctype_Info instance will be returned if the
     *                                   provided DOCTYPE HTML is a valid DOCTYPE. Otherwise, null.
     */
    pub fn from_doctype_token(doctype_html: &[u8]) -> Option<Self> {
        let mut doctype_name = None;
        let mut doctype_public_id = None;
        let mut doctype_system_id = None;

        /*
         * This parser combines the rules for parsing DOCTYPE tokens found in the HTML
         * specification for the DOCTYPE related tokenizer states.
         *
         * @see https://html.spec.whatwg.org/#doctype-state
         */

        /*
         * - Valid DOCTYPE HTML token must be at least `<!DOCTYPE>` assuming a complete token not
         *   ending in end-of-file.
         * - It must start with an ASCII case-insensitive match for `<!DOCTYPE`.
         * - The only occurrence of `>` must be the final byte in the HTML string.
         */
        if doctype_html.len() < 10 || !doctype_html[0..9].eq_ignore_ascii_case(b"<!DOCTYPE") {
            return None;
        }

        let mut at: usize = 9;
        // Is there one and only one `>`?
        if b'>' != doctype_html[doctype_html.len() - 1]
            || (strcspn!(doctype_html, b'>', at) + at) < doctype_html.len() - 1
        {
            return None;
        }

        /*
         * Perform newline normalization and ensure the $end value is correct after normalization.
         *
         * @see https://html.spec.whatwg.org/#preprocessing-the-input-stream
         * @see https://infra.spec.whatwg.org/#normalize-newlines
         */

        let mut doctype_html_normalized: Vec<u8> = Vec::new();
        let mut chars = doctype_html.iter().peekable();
        while let Some(&c) = chars.next() {
            match c {
                b'\r' => {
                    if chars.peek() == Some(&&b'\n') {
                        chars.next(); // consume the \n
                        doctype_html_normalized.push(b'\n');
                    } else {
                        doctype_html_normalized.push(b'\n');
                    }
                }
                b'\0' => {
                    "\u{FFFD}"
                        .as_bytes()
                        .iter()
                        .for_each(|c| doctype_html_normalized.push(*c));
                }
                _ => doctype_html_normalized.push(c),
            }
        }
        let doctype_html = doctype_html_normalized.as_slice();

        let end = doctype_html.len() - 1;

        /*
         * In this state, the doctype token has been found and its "content" optionally including the
         * name, public identifier, and system identifier is between the current position and the end.
         *
         *     "<!DOCTYPE...declaration...>"
         *               ╰─ $at           ╰─ $end
         *
         * It's also possible that the declaration part is empty.
         *
         *               ╭─ $at
         *     "<!DOCTYPE>"
         *               ╰─ $end
         *
         * Rules for parsing ">" which terminates the DOCTYPE do not need to be considered as they
         * have been handled above in the condition that the provided DOCTYPE HTML must contain
         * exactly one ">" character in the final position.
         */

        /*
         *
         * Parsing effectively begins in "Before DOCTYPE name state". Ignore whitespace and
         * proceed to the next state.
         *
         * @see https://html.spec.whatwg.org/#before-doctype-name-state
         */
        at += strspn!(doctype_html, b' ' | b'\t' | b'\n' | 0x0c | b'\r', at);

        if at >= end {
            return Some(Self::new(
                doctype_name,
                doctype_public_id,
                doctype_system_id,
                true,
            ));
        }

        let name_length = strcspn!(
            doctype_html,
            b' ' | b'\t' | b'\n' | 0x0c | b'\r',
            at,
            end - at
        );
        doctype_name = Some(
            doctype_html[at..at + name_length]
                .to_ascii_lowercase()
                .into(),
        );

        at += name_length;
        at += strspn!(
            doctype_html,
            b' ' | b'\t' | b'\n' | 0x0c | b'\r',
            at,
            end - at
        );
        if at >= end {
            return Some(Self::new(
                doctype_name,
                doctype_public_id,
                doctype_system_id,
                false,
            ));
        }

        /*
         * "After DOCTYPE name state"
         *
         * Find a case-insensitive match for "PUBLIC" or "SYSTEM" at this point.
         * Otherwise, set force-quirks and enter bogus DOCTYPE state (skip the rest of the doctype).
         *
         * @see https://html.spec.whatwg.org/#after-doctype-name-state
         */
        if at + 6 >= end {
            return Some(Self::new(
                doctype_name,
                doctype_public_id,
                doctype_system_id,
                true,
            ));
        }

        /*
         * > If the six characters starting from the current input character are an ASCII
         * > case-insensitive match for the word "PUBLIC", then consume those characters
         * > and switch to the after DOCTYPE public keyword state.
         */
        let mut next_parse = if doctype_html[at..at + 6].eq_ignore_ascii_case(b"PUBLIC") {
            at += 6;
            at += strspn!(
                doctype_html,
                b' ' | b'\t' | b'\n' | 0x0c | b'\r',
                at,
                end - at
            );
            if at >= end {
                return Some(Self::new(
                    doctype_name,
                    doctype_public_id,
                    doctype_system_id,
                    true,
                ));
            }
            Proceed::ParseDoctypePublicIdentifier
        }
        /*
         * > Otherwise, if the six characters starting from the current input character are an ASCII
         * > case-insensitive match for the word "SYSTEM", then consume those characters and switch
         * > to the after DOCTYPE system keyword state.
         */
        else if doctype_html[at..at + 6].eq_ignore_ascii_case(b"SYSTEM") {
            at += 6;
            at += strspn!(
                doctype_html,
                b' ' | b'\t' | b'\n' | 0x0c | b'\r',
                at,
                end - at
            );
            if at >= end {
                return Some(Self::new(
                    doctype_name,
                    doctype_public_id,
                    doctype_system_id,
                    true,
                ));
            }
            Proceed::ParseDoctypeSystemIdentifier
        } else {
            /*
             * > Otherwise, this is an invalid-character-sequence-after-doctype-name parse error.
             * > Set the current DOCTYPE token's force-quirks flag to on. Reconsume in the bogus
             * > DOCTYPE state.
             */
            return Some(Self::new(
                doctype_name,
                doctype_public_id,
                doctype_system_id,
                true,
            ));
        };

        loop {
            next_parse = match next_parse {
                // GOTO TARGET parse_doctype_public_identifier:
                Proceed::ParseDoctypePublicIdentifier => {
                    /*
                     * The parser should enter "DOCTYPE public identifier (double-quoted) state" or
                     * "DOCTYPE public identifier (single-quoted) state" by finding one of the valid quotes.
                     * Anything else forces quirks mode and ignores the rest of the contents.
                     *
                     * @see https://html.spec.whatwg.org/#doctype-public-identifier-(double-quoted)-state
                     * @see https://html.spec.whatwg.org/#doctype-public-identifier-(single-quoted)-state
                     */
                    let closer_quote = doctype_html[at];

                    /*
                     * > This is a missing-quote-before-doctype-public-identifier parse error. Set the
                     * > current DOCTYPE token's force-quirks flag to on. Reconsume in the bogus DOCTYPE state.
                     */
                    if b'"' != closer_quote && b'\'' != closer_quote {
                        return Some(Self::new(
                            doctype_name,
                            doctype_public_id,
                            doctype_system_id,
                            true,
                        ));
                    }

                    at += 1;

                    let identifier_length =
                        strcspn!(doctype_html, x if x == closer_quote, at, end - at);

                    doctype_public_id = Some(doctype_html[at..at + identifier_length].into());

                    at += identifier_length;
                    if at >= end || closer_quote != doctype_html[at] {
                        return Some(Self::new(
                            doctype_name,
                            doctype_public_id,
                            doctype_system_id,
                            true,
                        ));
                    }

                    at += 1;

                    /*
                     * "Between DOCTYPE public and system identifiers state"
                     *
                     * Advance through whitespace between public and system identifiers.
                     *
                     * @see https://html.spec.whatwg.org/#between-doctype-public-and-system-identifiers-state
                     */
                    at += strspn!(
                        doctype_html,
                        b' ' | b'\t' | b'\n' | 0x0c | b'\r',
                        at,
                        end - at
                    );
                    if at >= end {
                        return Some(Self::new(
                            doctype_name,
                            doctype_public_id,
                            doctype_system_id,
                            false,
                        ));
                    }
                    Proceed::ParseDoctypeSystemIdentifier
                }
                // GOTO TARGET parse_doctype_system_identifier:
                Proceed::ParseDoctypeSystemIdentifier => {
                    /*
                     * The parser should enter "DOCTYPE system identifier (double-quoted) state" or
                     * "DOCTYPE system identifier (single-quoted) state" by finding one of the valid quotes.
                     * Anything else forces quirks mode and ignores the rest of the contents.
                     *
                     * @see https://html.spec.whatwg.org/#doctype-system-identifier-(double-quoted)-state
                     * @see https://html.spec.whatwg.org/#doctype-system-identifier-(single-quoted)-state
                     */
                    let closer_quote = doctype_html[at];

                    /*
                     * > This is a missing-quote-before-doctype-system-identifier parse error. Set the
                     * > current DOCTYPE token's force-quirks flag to on. Reconsume in the bogus DOCTYPE state.
                     */
                    if b'"' != closer_quote && b'\'' != closer_quote {
                        return Some(Self::new(
                            doctype_name,
                            doctype_public_id,
                            doctype_system_id,
                            true,
                        ));
                    }

                    at += 1;

                    let identifier_length =
                        strcspn!(doctype_html, x if x == closer_quote, at, end - at);
                    doctype_system_id = Some(doctype_html[at..at + identifier_length].into());

                    at += identifier_length;
                    if at >= end || closer_quote != doctype_html[at] {
                        return Some(Self::new(
                            doctype_name,
                            doctype_public_id,
                            doctype_system_id,
                            true,
                        ));
                    }
                    Proceed::Exit
                }
                Proceed::Exit => break,
            }
        }

        return Some(Self::new(
            doctype_name,
            doctype_public_id,
            doctype_system_id,
            false,
        ));
    }
}

enum Proceed {
    ParseDoctypePublicIdentifier,
    ParseDoctypeSystemIdentifier,
    Exit,
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_doctype_info {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (a,b,c,d,e): (&str, CompatMode,Option<&str>,Option<&str>,Option<&str>) = $value;
                let doctype = HtmlDoctypeInfo::from_doctype_token(a.as_bytes());
                assert!(
                    doctype.is_some(),
                    "Should have parsed the following doctype declaration: {:?}",
                    String::from_utf8_lossy(a.as_bytes())
                );
                let doctype = doctype.unwrap();

                assert_eq!(
                    b,
                    doctype.indicated_compatability_mode,
                    "Failed to infer the expected document compatability mode for {:?}",
                    String::from_utf8_lossy(a.as_bytes())
                );

                assert_eq!(
                    c.map(|val| val.as_bytes().into()),
                    doctype.name,
                    "Failed to parse the expected DOCTYPE name for {:?}",
                    String::from_utf8_lossy(a.as_bytes())
                );

                assert_eq!(
                    d.map(|val| val.as_bytes().into()),
                    doctype.public_identifier,
                    "Failed to parse the expected DOCTYPE public identifier for {:?}",
                    String::from_utf8_lossy(a.as_bytes())
                );

                assert_eq!(
                    e.map(|val| val.as_bytes().into()),
                    doctype.system_identifier,
                    "Failed to parse the expected DOCTYPE system identifier for {:?}",
                    String::from_utf8_lossy(a.as_bytes())
                );
            }
        )*
        }
    }

    test_doctype_info! {
        missing_doctype_name:                                  ( "<!DOCTYPE>",                                                                                              CompatMode::NoQuirks,      None,                                                                    None,                                     None ),
        html5_doctype:                                         ( "<!DOCTYPE html>",                                                                                         CompatMode::NoQuirks,      Some("html"),                                                            None,                                     None ),
        html5_doctype_no_whitespace_before_name:               ( "<!DOCTYPEhtml>",                                                                                          CompatMode::NoQuirks,      Some("html"),                                                            None,                                     None ),
        xhtml_doctype:                                         ( r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#,           CompatMode::NoQuirks,      Some("html"),                                                            Some("-//W3C//DTD HTML 4.01//EN"),        Some("http://www.w3.org/TR/html4/strict.dtd") ),
        svg_doctype:                                           ( r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#,   CompatMode::Quirks,        Some("svg"),                                                             Some("-//W3C//DTD SVG 1.1//EN"),          Some("http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd") ),
        mathml_doctype:                                        ( r#"<!DOCTYPE math PUBLIC "-//W3C//DTD MathML 2.0//EN" "http://www.w3.org/Math/DTD/mathml2/mathml2.dtd">"#, CompatMode::Quirks,        Some("math"),                                                            Some("-//W3C//DTD MathML 2.0//EN"),       Some("http://www.w3.org/Math/DTD/mathml2/mathml2.dtd") ),
        doctype_with_null_byte_replacement:                    ( "<!DOCTYPE null-\0 PUBLIC '\0' '\0\0'>",                                                                   CompatMode::Quirks,        Some("null-\u{FFFD}"),                                                   Some("\u{FFFD}"),                         Some("\u{FFFD}\u{FFFD}") ),
        uppercase_doctype:                                     ( "<!DOCTYPE UPPERCASE>",                                                                                    CompatMode::Quirks,        Some("uppercase"),                                                       None,                                     None ),
        lowercase_doctype:                                     ( "<!doctype lowercase>",                                                                                    CompatMode::Quirks,        Some("lowercase"),                                                       None,                                     None ),
        doctype_with_whitespace:                               ( "<!DOCTYPE\n\thtml\x0c\rPUBLIC\r\n''\t''>",                                                                CompatMode::NoQuirks,      Some("html"),                                                            Some(""),                                 Some("") ),
        doctype_trailing_characters:                           ( "<!DOCTYPE html PUBLIC '' '' Anything (except closing angle bracket) is just fine here !!!>",              CompatMode::NoQuirks,      Some("html"),                                                            Some(""),                                 Some("") ),
        an_ugly_no_quirks_doctype:                             ( "<!dOcTyPehtml\tPublIC\"pub-id\"'sysid'>",                                                                 CompatMode::NoQuirks,      Some("html"),                                                            Some("pub-id"),                           Some("sysid") ),
        missing_public_id:                                     ( "<!DOCTYPE html PUBLIC>",                                                                                  CompatMode::Quirks,        Some("html"),                                                            None,                                     None ),
        missing_system_id:                                     ( "<!DOCTYPE html SYSTEM>",                                                                                  CompatMode::Quirks,        Some("html"),                                                            None,                                     None ),
        missing_close_quote_public_id:                         ( "<!DOCTYPE html PUBLIC 'xyz>",                                                                             CompatMode::Quirks,        Some("html"),                                                            Some("xyz"),                              None ),
        missing_close_quote_system_id:                         ( r#"<!DOCTYPE html SYSTEM "xyz>"#,                                                                          CompatMode::Quirks,        Some("html"),                                                            None,                                     Some("xyz") ),
        missing_close_quote_system_id_with_public:             ( "<!DOCTYPE html PUBLIC 'abc' 'xyz>",                                                                       CompatMode::Quirks,        Some("html"),                                                            Some("abc"),                              Some("xyz") ),
        bogus_characters_instead_of_system_or_public:          ( "<!DOCTYPE html FOOBAR>",                                                                                  CompatMode::Quirks,        Some("html"),                                                            None,                                     None ),
        bogus_characters_instead_of_public_quote:              ( "<!DOCTYPE html PUBLIC x ''''>",                                                                           CompatMode::Quirks,        Some("html"),                                                            None,                                     None ),
        bogus_characters_instead_of_system_quote_:             ( "<!DOCTYPE html SYSTEM x ''>",                                                                             CompatMode::Quirks,        Some("html"),                                                            None,                                     None ),
        emoji:                                                 ( r#"<!DOCTYPE 🏴󠁧󠁢󠁥󠁮󠁧󠁿 PUBLIC "🔥" "😈">"#,                                                                 CompatMode::Quirks,        Some("\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}"), Some("🔥"),                               Some("😈") ),
        bogus_characters_instead_of_system_quote_after_public: ( "<!DOCTYPE html PUBLIC ''x''>",                                                                            CompatMode::Quirks,        Some("html"),                                                            Some(""),                                 None ),
        special_quirks_mode_if_system_unset:                   ( r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Frameset//">"#,                                            CompatMode::Quirks,        Some("html"),                                                            Some("-//W3C//DTD HTML 4.01 Frameset//"), None ),
        special_limited_quirks_mode_if_system_set:             ( r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01 Frameset//" "">"#,                                         CompatMode::LimitedQuirks, Some("html"),                                                            Some("-//W3C//DTD HTML 4.01 Frameset//"), Some("") ),
    }

    #[test]
    fn test_invalid_inputs() {
        let test_cases = vec![
            b"".as_slice(),
            b"<div>".as_slice(),
            b"x<!DOCTYPE>".as_slice(),
            b"<!DOCTYPE>x".as_slice(),
            b"<!DOCTYPE".as_slice(),
            b"<!DOCTYPE html PUBLIC \">\">".as_slice(),
        ];

        for html in test_cases {
            assert!(
                HtmlDoctypeInfo::from_doctype_token(html).is_none(),
                "Should return None for invalid input: {:?}",
                String::from_utf8_lossy(html)
            );
        }
    }
}
