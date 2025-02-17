/// Insertion mode.
///
/// @see https://html.spec.whatwg.org/#the-insertion-mode
#[derive(Debug, PartialEq, Clone)]
pub(super) enum InsertionMode {
    /// Initial insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#the-initial-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    INITIAL,

    /// Before HTML insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#the-before-html-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    BEFORE_HTML,

    /// Before head insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-beforehead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    BEFORE_HEAD,

    /// In head insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inhead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_HEAD,

    /// In head noscript insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inheadnoscript
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_HEAD_NOSCRIPT,

    /// After head insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterhead
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_HEAD,

    /// In body insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inbody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_BODY,

    /// In table insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intable
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE,

    /// In table text insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intabletext
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE_TEXT,

    /// In caption insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incaption
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_CAPTION,

    /// In column group insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incolumngroup
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_COLUMN_GROUP,

    /// In table body insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intablebody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TABLE_BODY,

    /// In row insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inrow
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_ROW,

    /// In cell insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-incell
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_CELL,

    /// In select insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselect
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_SELECT,

    /// In select in table insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inselectintable
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_SELECT_IN_TABLE,

    /// In template insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-intemplate
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_TEMPLATE,

    /// After body insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterbody
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_BODY,

    /// In frameset insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-inframeset
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    IN_FRAMESET,

    /// After frameset insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#parsing-main-afterframeset
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_FRAMESET,

    /// After after body insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-body-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_AFTER_BODY,

    /// After after frameset insertion mode for full HTML parser.
    ///
    /// @see https://html.spec.whatwg.org/#the-after-after-frameset-insertion-mode
    /// @see WP_HTML_Processor_State::$insertion_mode
    ///
    /// @var string
    AFTER_AFTER_FRAMESET,
}
