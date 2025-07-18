#![allow(non_camel_case_types)]

use ext_php_rs::{
    binary::Binary,
    binary_slice::BinarySlice,
    builders::ModuleBuilder,
    convert::{FromZval, IntoZval},
    prelude::*,
    types::{ZendClassObject, Zval},
    zend::Function,
};
use std::ops::Deref;
use wp_html_api::tag_processor::{
    AttributeValue, NextTagQuery, NodeName, ParserState, ParsingNamespace, TagClosers,
    TagProcessor, TokenType,
};
use wp_html_api::{doctype::HtmlDoctypeInfo, tag_name::TagName};
use wp_html_api::{html_processor::HtmlProcessor, tag_processor::CommentType};

extern "C" fn startup_function(_ty: i32, _module_number: i32) -> i32 {
    0
}

#[php_class]
#[php(change_case = "none")]
pub struct WP_HTML_Tag_Processor {
    processor: TagProcessor,
}

#[php_impl]
#[php(change_method_case = "none", change_constant_case = "none")]
impl WP_HTML_Tag_Processor {
    pub fn __construct(html: BinarySlice<u8>) -> Self {
        let processor = TagProcessor::new(html.clone());
        Self { processor }
    }

    pub fn get_updated_html(&self) -> Binary<u8> {
        Binary::from(self.processor.get_updated_html().as_ref().to_vec())
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn set_bookmark(&mut self, name: &str) -> bool {
        self.processor.set_bookmark(name).is_ok()
    }

    pub fn next_tag(&mut self, query: Option<PhpNextTagQuery>) -> bool {
        self.processor.next_tag(query.map(Into::into))
    }

    pub fn class_list(&self) -> Vec<Binary<u8>> {
        self.processor
            .class_list()
            .map(|class_name| class_name.to_vec().into())
            .collect()
    }

    pub fn has_class(&self, wanted_class: &str) -> Option<bool> {
        self.processor.has_class(wanted_class)
    }

    pub fn is_tag_closer(&self) -> bool {
        self.processor.is_tag_closer()
    }

    pub fn has_self_closing_flag(&self) -> bool {
        self.processor.has_self_closing_flag()
    }

    pub fn get_doctype_info(&self) -> Option<WP_HTML_Doctype_Info> {
        self.processor
            .get_doctype_info()
            .map(|internal| WP_HTML_Doctype_Info { internal })
    }

    pub fn get_tag(&self) -> Option<Binary<u8>> {
        self.processor.get_tag().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_tag_name(&self) -> Option<Binary<u8>> {
        self.processor.get_qualified_tag_name().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_attribute_name(
        &self,
        attribute_name: BinarySlice<u8>,
    ) -> Option<Binary<u8>> {
        self.processor
            .get_qualified_attribute_name(&attribute_name)
            .map(|tag_name| {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            })
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(&self) -> Option<Binary<u8>> {
        self.processor.get_token_name().map(|name| match name {
            NodeName::Tag(tag_name) => {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            }
            NodeName::Token(token_name) => {
                if token_name == TokenType::Doctype {
                    b"html".to_vec().into()
                } else {
                    let token_name: String = token_name.into();
                    token_name.as_bytes().to_vec().into()
                }
            }
        })
    }

    pub fn get_modifiable_text(&self) -> Binary<u8> {
        self.processor.get_modifiable_text().to_vec().into()
    }

    pub fn set_modifiable_text(&mut self, updated_text: String) -> bool {
        self.processor.set_modifiable_text(updated_text.as_str())
    }

    pub fn get_attribute(&self, prefix: BinarySlice<u8>) -> Option<AttributeValueWrapper> {
        self.processor
            .get_attribute(&prefix)
            .map(|val| AttributeValueWrapper(val))
    }

    pub fn get_attribute_names_with_prefix(
        &self,
        prefix: BinarySlice<u8>,
    ) -> Option<Vec<Binary<u8>>> {
        self.processor
            .get_attribute_names_with_prefix(&prefix)
            .map(|vec| {
                vec.iter()
                    .map(|name| name.to_vec().into())
                    .collect::<Vec<Binary<u8>>>()
            })
    }

    pub fn paused_at_incomplete_token(&self) -> bool {
        self.processor.paused_at_incomplete_token()
    }

    pub fn get_comment_type(&self) -> Option<Binary<u8>> {
        self.processor.get_comment_type().map(|comment_type| {
            match comment_type {
                CommentType::AbruptlyClosedComment => COMMENT_AS_ABRUPTLY_CLOSED_COMMENT,
                CommentType::CdataLookalike => COMMENT_AS_CDATA_LOOKALIKE,
                CommentType::HtmlComment => COMMENT_AS_HTML_COMMENT,
                CommentType::PiNodeLookalike => COMMENT_AS_PI_NODE_LOOKALIKE,
                CommentType::InvalidHtml => COMMENT_AS_INVALID_HTML,
            }
            .bytes()
            .collect::<Vec<_>>()
            .into()
        })
    }

    pub fn get_full_comment_text(&self) -> Option<Binary<u8>> {
        self.processor
            .get_full_comment_text()
            .map(|value| value.to_vec().into())
    }

    pub fn subdivide_text_appropriately(&mut self) -> bool {
        self.processor.subdivide_text_appropriately()
    }

    pub fn change_parsing_namespace(&mut self, new_namespace: BinarySlice<u8>) -> bool {
        match *new_namespace {
            b"html" => {
                self.processor
                    .change_parsing_namespace(ParsingNamespace::Html);
                true
            }
            b"math" => {
                self.processor
                    .change_parsing_namespace(ParsingNamespace::MathML);
                true
            }
            b"svg" => {
                self.processor
                    .change_parsing_namespace(ParsingNamespace::Svg);
                true
            }
            _ => false,
        }
    }

    #[php(getter)]
    #[php(vis = "protected")]
    fn parser_state(&self) -> Binary<u8> {
        match self.processor.parser_state() {
            ParserState::Ready => STATE_READY,
            ParserState::Complete => STATE_COMPLETE,
            ParserState::IncompleteInput => STATE_INCOMPLETE_INPUT,
            ParserState::MatchedTag => STATE_MATCHED_TAG,
            ParserState::TextNode => STATE_TEXT_NODE,
            ParserState::CDATANode => STATE_CDATA_NODE,
            ParserState::Comment => STATE_COMMENT,
            ParserState::Doctype => STATE_DOCTYPE,
            ParserState::PresumptuousTag => STATE_PRESUMPTUOUS_TAG,
            ParserState::FunkyComment => STATE_FUNKY_COMMENT,
        }
        .bytes()
        .collect::<Vec<_>>()
        .into()
    }

    pub const STATE_READY: &str = STATE_READY;
    pub const STATE_COMPLETE: &str = STATE_COMPLETE;
    pub const STATE_INCOMPLETE_INPUT: &str = STATE_INCOMPLETE_INPUT;
    pub const STATE_MATCHED_TAG: &str = STATE_MATCHED_TAG;
    pub const STATE_TEXT_NODE: &str = STATE_TEXT_NODE;
    pub const STATE_CDATA_NODE: &str = STATE_CDATA_NODE;
    pub const STATE_COMMENT: &str = STATE_COMMENT;
    pub const STATE_DOCTYPE: &str = STATE_DOCTYPE;
    pub const STATE_PRESUMPTUOUS_TAG: &str = STATE_PRESUMPTUOUS_TAG;
    pub const STATE_FUNKY_COMMENT: &str = STATE_FUNKY_COMMENT;

    pub const COMMENT_AS_ABRUPTLY_CLOSED_COMMENT: &str = COMMENT_AS_ABRUPTLY_CLOSED_COMMENT;
    pub const COMMENT_AS_CDATA_LOOKALIKE: &str = COMMENT_AS_CDATA_LOOKALIKE;
    pub const COMMENT_AS_HTML_COMMENT: &str = COMMENT_AS_HTML_COMMENT;
    pub const COMMENT_AS_PI_NODE_LOOKALIKE: &str = COMMENT_AS_PI_NODE_LOOKALIKE;
    pub const COMMENT_AS_INVALID_HTML: &str = COMMENT_AS_INVALID_HTML;
}

/*
 * Constants from PHP classes.
 */
const STATE_READY: &str = "STATE_READY";
const STATE_COMPLETE: &str = "STATE_COMPLETE";
const STATE_INCOMPLETE_INPUT: &str = "STATE_INCOMPLETE_INPUT";
const STATE_MATCHED_TAG: &str = "STATE_MATCHED_TAG";
const STATE_TEXT_NODE: &str = "STATE_TEXT_NODE";
const STATE_CDATA_NODE: &str = "STATE_CDATA_NODE";
const STATE_COMMENT: &str = "STATE_COMMENT";
const STATE_DOCTYPE: &str = "STATE_DOCTYPE";
const STATE_PRESUMPTUOUS_TAG: &str = "STATE_PRESUMPTUOUS_TAG";
const STATE_FUNKY_COMMENT: &str = "STATE_WP_FUNKY";

const COMMENT_AS_ABRUPTLY_CLOSED_COMMENT: &str = "COMMENT_AS_ABRUPTLY_CLOSED_COMMENT";
const COMMENT_AS_CDATA_LOOKALIKE: &str = "COMMENT_AS_CDATA_LOOKALIKE";
const COMMENT_AS_HTML_COMMENT: &str = "COMMENT_AS_HTML_COMMENT";
const COMMENT_AS_PI_NODE_LOOKALIKE: &str = "COMMENT_AS_PI_NODE_LOOKALIKE";
const COMMENT_AS_INVALID_HTML: &str = "COMMENT_AS_INVALID_HTML";

const NO_QUIRKS_MODE: &str = "no-quirks-mode";
const QUIRKS_MODE: &str = "quirks-mode";

struct AttributeValueWrapper(AttributeValue);
impl IntoZval for AttributeValueWrapper {
    const NULLABLE: bool = true;
    const TYPE: ext_php_rs::flags::DataType = ext_php_rs::flags::DataType::Mixed;

    fn set_zval(
        self,
        zv: &mut ext_php_rs::types::Zval,
        _persistent: bool,
    ) -> ext_php_rs::error::Result<()> {
        Ok(match self.0 {
            AttributeValue::BooleanFalse => zv.set_null(),
            AttributeValue::BooleanTrue => zv.set_bool(true),
            AttributeValue::String(value) => zv.set_binary(value.to_vec().into()),
        })
    }
}

/// Wrapper struct for NextTagQuery to implement FromZval
struct PhpNextTagQuery(NextTagQuery);

impl Deref for PhpNextTagQuery {
    type Target = NextTagQuery;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<NextTagQuery> for PhpNextTagQuery {
    fn into(self) -> NextTagQuery {
        self.0
    }
}

impl<'a> FromZval<'a> for PhpNextTagQuery {
    const TYPE: ext_php_rs::flags::DataType = ext_php_rs::flags::DataType::Mixed;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        // Simple string query (tag name)
        if zval.is_string() {
            let tag_name = zval.binary().expect("Expected a string");
            // NextTagQuery directly implements From<&[u8]>
            let tag_name: TagName = (tag_name.as_slice(), &ParsingNamespace::Html).into();
            return Some(PhpNextTagQuery(NextTagQuery {
                tag_name: Some(tag_name),
                ..Default::default()
            }));
        }

        // Array-based query
        if zval.is_array() {
            let array = zval.array().expect("Expected an array");
            let mut next_tag_query = NextTagQuery::default();

            // Process tag_name if provided
            if let Some(tag_name) = array.get("tag_name") {
                if tag_name.is_string() {
                    if let Some(tag_name_str) = tag_name.binary() {
                        // We need to explicitly construct the TagName from a slice and namespace
                        next_tag_query.tag_name =
                            Some((tag_name_str.as_slice(), &ParsingNamespace::Html).into());
                    }
                }
            }

            // Process match_offset if provided
            if let Some(match_offset) = array.get("match_offset") {
                if match_offset.is_long() {
                    if let Some(offset) = match_offset.long() {
                        // Ensure offset is positive and convert to usize
                        if offset > 0 {
                            next_tag_query.match_offset = offset as usize;
                        }
                    }
                }
            }

            // Process class_name if provided
            if let Some(class_name) = array.get("class_name") {
                if class_name.is_string() {
                    if let Some(class_name_str) = class_name.binary() {
                        next_tag_query.class_name =
                            Some(String::from_utf8_lossy(class_name_str.as_slice()).to_string());
                    }
                }
            }

            // Process tag_closers if provided
            if let Some(tag_closers) = array.get("tag_closers") {
                if tag_closers.is_string() {
                    if let Some(tag_closers_str) = tag_closers.binary() {
                        if tag_closers_str.as_slice().eq_ignore_ascii_case(b"visit") {
                            next_tag_query.tag_closers = TagClosers::Visit;
                        }
                    }
                }
            }

            return Some(PhpNextTagQuery(next_tag_query));
        }

        // Default to an empty query
        Some(PhpNextTagQuery(NextTagQuery::default()))
    }
}

#[php_class]
#[php(change_case = "none")]
pub struct WP_HTML_Processor {
    processor: HtmlProcessor,
}

#[php_impl]
#[php(change_method_case = "none", change_constant_case = "none")]
impl WP_HTML_Processor {
    pub fn __construct(
        html: BinarySlice<u8>,
        _use_the_static_create_methods_instead: Option<BinarySlice<u8>>,
    ) -> Result<Self, PhpException> {
        if let Some(_doing_it_wrong) = Function::try_from_function("_doing_it_wrong") {
            println!("doing it wrong");
            _doing_it_wrong.try_call(
                vec![
                    &"WP_HTML_Processor::__construct",
                    &"Call <code>WP_HTML_Processor::create_fragment()</code> to create an HTML Processor instead of calling the constructor directly.",
                ],
            )?;
        }
        if let Some(processor) = Self::create_fragment(html, None, None) {
            Ok(processor)
        } else {
            Err(PhpException::from("Could not create HTML Processor"))
        }
    }

    pub fn create_fragment(
        html: BinarySlice<u8>,
        context: Option<&str>,
        encoding: Option<&str>,
    ) -> Option<Self> {
        let context = context.unwrap_or("<body>");
        let encoding = encoding.unwrap_or("UTF-8");
        HtmlProcessor::create_fragment(html.clone(), context, encoding)
            .map(|processor| Self { processor })
    }

    pub fn create_full_parser(
        html: BinarySlice<u8>,
        known_definite_encoding: Option<&str>,
    ) -> Option<Self> {
        let known_definite_encoding = known_definite_encoding.unwrap_or("UTF-8");
        HtmlProcessor::create_full_parser(html.clone(), known_definite_encoding)
            .map(|processor| Self { processor })
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn get_tag(&self) -> Option<Binary<u8>> {
        self.processor.get_tag().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_tag_name(&self) -> Option<Binary<u8>> {
        self.processor.get_qualified_tag_name().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_attribute_name(
        &self,
        attribute_name: BinarySlice<u8>,
    ) -> Option<Binary<u8>> {
        self.processor
            .get_qualified_attribute_name(&attribute_name)
            .map(|name| name.to_vec().into())
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(&self) -> Option<Binary<u8>> {
        self.processor.get_token_name().map(|name| match name {
            NodeName::Tag(tag_name) => {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            }
            NodeName::Token(token_name) => {
                if token_name == TokenType::Doctype {
                    b"html".to_vec().into()
                } else {
                    let token_name: String = token_name.into();
                    token_name.as_bytes().to_vec().into()
                }
            }
        })
    }

    pub fn get_attribute_names_with_prefix(
        &self,
        prefix: BinarySlice<u8>,
    ) -> Option<Vec<Binary<u8>>> {
        self.processor
            .get_attribute_names_with_prefix(&prefix)
            .map(|vec| {
                vec.iter()
                    .map(|name| name.to_vec().into())
                    .collect::<Vec<Binary<u8>>>()
            })
    }

    pub fn get_attribute(&self, prefix: BinarySlice<u8>) -> Option<AttributeValueWrapper> {
        self.processor
            .get_attribute(&prefix)
            .map(|val| AttributeValueWrapper(val))
    }

    pub fn get_last_error(&self) -> Option<String> {
        self.processor.get_last_error().map(|value| {
            let s: &str = value.into();
            s.to_owned()
        })
    }

    // Fix this to _return_ a nullable value.
    pub fn get_unsupported_exception(&self) -> Result<(), &str> {
        match self.processor.get_unsupported_exception() {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }

    pub fn is_tag_closer(&self) -> bool {
        self.processor.is_tag_closer()
    }

    pub fn get_namespace(&self) -> String {
        self.processor.get_namespace().into()
    }

    pub fn expects_closer(&self) -> Option<bool> {
        self.processor.expects_closer(None)
    }

    pub fn get_modifiable_text(&self) -> Binary<u8> {
        self.processor.get_modifiable_text().to_vec().into()
    }

    pub fn get_doctype_info(&self) -> Option<WP_HTML_Doctype_Info> {
        self.processor
            .get_doctype_info()
            .map(|internal| WP_HTML_Doctype_Info { internal })
    }

    pub fn paused_at_incomplete_token(&self) -> bool {
        self.processor.paused_at_incomplete_token()
    }

    pub fn get_full_comment_text(&self) -> Option<Binary<u8>> {
        self.processor
            .get_full_comment_text()
            .map(|value| value.to_vec().into())
    }

    pub fn set_bookmark(&mut self, name: &str) -> bool {
        self.processor.set_bookmark(name).is_ok()
    }

    pub fn is_void(name: BinarySlice<u8>) -> bool {
        let name = TagName::from((name.into(), &ParsingNamespace::Html));
        HtmlProcessor::is_void(&name)
    }

    pub const STATE_READY: &str = STATE_READY;
    pub const STATE_COMPLETE: &str = STATE_COMPLETE;
    pub const STATE_INCOMPLETE_INPUT: &str = STATE_INCOMPLETE_INPUT;
    pub const STATE_MATCHED_TAG: &str = STATE_MATCHED_TAG;
    pub const STATE_TEXT_NODE: &str = STATE_TEXT_NODE;
    pub const STATE_CDATA_NODE: &str = STATE_CDATA_NODE;
    pub const STATE_COMMENT: &str = STATE_COMMENT;
    pub const STATE_DOCTYPE: &str = STATE_DOCTYPE;
    pub const STATE_PRESUMPTUOUS_TAG: &str = STATE_PRESUMPTUOUS_TAG;
    pub const STATE_FUNKY_COMMENT: &str = STATE_FUNKY_COMMENT;

    pub const COMMENT_AS_ABRUPTLY_CLOSED_COMMENT: &str = COMMENT_AS_ABRUPTLY_CLOSED_COMMENT;
    pub const COMMENT_AS_CDATA_LOOKALIKE: &str = COMMENT_AS_CDATA_LOOKALIKE;
    pub const COMMENT_AS_HTML_COMMENT: &str = COMMENT_AS_HTML_COMMENT;
    pub const COMMENT_AS_PI_NODE_LOOKALIKE: &str = COMMENT_AS_PI_NODE_LOOKALIKE;
    pub const COMMENT_AS_INVALID_HTML: &str = COMMENT_AS_INVALID_HTML;

    pub const CONSTRUCTOR_UNLOCK_CODE: &str = "Use WP_HTML_Processor::create_fragment() instead of calling the class constructor directly.";
}

#[php_class]
#[php(change_case = "none")]
struct WP_HTML_Text_Replacement {
    start: usize,
    length: usize,
    text: Binary<u8>,
}

#[php_class]
#[php(change_case = "none")]
pub struct WP_HTML_Doctype_Info {
    internal: HtmlDoctypeInfo,
}

#[php_impl]
#[php(change_method_case = "none", change_constant_case = "none")]
impl WP_HTML_Doctype_Info {
    pub fn from_doctype_token(html: BinarySlice<u8>) -> Option<Self> {
        let html = html.to_vec();
        HtmlDoctypeInfo::from_doctype_token(&html).map(|internal| Self { internal })
    }

    #[php(getter)]
    pub fn name(&self) -> Option<Binary<u8>> {
        self.internal.name.as_ref().map(|val| val.to_vec().into())
    }

    #[php(getter)]
    pub fn public_identifier(&self) -> Option<Binary<u8>> {
        self.internal
            .public_identifier
            .as_ref()
            .map(|val| val.to_vec().into())
    }

    #[php(getter)]
    pub fn system_identifier(&self) -> Option<Binary<u8>> {
        self.internal
            .system_identifier
            .as_ref()
            .map(|val| val.to_vec().into())
    }

    #[php(getter)]
    pub fn indicated_compatability_mode(&self) -> String {
        (&self.internal.indicated_compatability_mode).into()
    }
}

#[php_module]
#[php(startup = "startup_function")]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<WP_HTML_Doctype_Info>()
        .class::<WP_HTML_Tag_Processor>()
        .class::<WP_HTML_Processor>()
        .class::<WP_HTML_Text_Replacement>()
}
