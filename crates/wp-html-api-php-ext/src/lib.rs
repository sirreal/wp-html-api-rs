#![allow(non_camel_case_types)]

use ext_php_rs::convert::IntoZval;
use ext_php_rs::{
    binary::Binary, binary_slice::BinarySlice, builders::ModuleBuilder, prelude::*,
    types::ZendClassObject,
};
use wp_html_api::doctype::HtmlDoctypeInfo;
use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::{AttributeValue, NodeName, TagProcessor};

extern "C" fn request_startup(_ty: i32, _module_number: i32) -> i32 {
    0
}

extern "C" fn request_shutdown(_ty: i32, _module_number: i32) -> i32 {
    0
}

#[php_class]
pub struct WP_HTML_Tag_Processor {
    processor: TagProcessor,
}

#[php_impl(rename_methods = "none")]
impl WP_HTML_Tag_Processor {
    pub fn __construct(html: BinarySlice<u8>) -> Self {
        let processor = TagProcessor::new(html.clone());
        Self { processor }
    }

    pub fn get_updated_html(#[this] this: &mut ZendClassObject<Self>) -> Binary<u8> {
        Binary::from(this.processor.get_updated_html().as_ref().to_vec())
    }

    pub fn next_token(#[this] this: &mut ZendClassObject<Self>) -> bool {
        this.processor.next_token()
    }

    pub fn get_tag(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor.get_tag().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_tag_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor.get_qualified_tag_name().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_attribute_name(
        #[this] this: &mut ZendClassObject<Self>,
        attribute_name: BinarySlice<u8>,
    ) -> Option<Binary<u8>> {
        this.processor
            .get_qualified_attribute_name(&attribute_name)
            .map(|tag_name| {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            })
    }

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        let get_token_name = this.processor.get_token_name()?;
        Some(match get_token_name {
            NodeName::Tag(tag_name) => {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            }
            NodeName::Token(token_name) => {
                let token_name: String = token_name.into();
                token_name.as_bytes().to_vec().into()
            }
        })
    }

    pub fn get_modifiable_text(#[this] this: &mut ZendClassObject<Self>) -> Binary<u8> {
        this.processor.get_modifiable_text().to_vec().into()
    }

    pub fn set_modifiable_text(
        #[this] this: &mut ZendClassObject<Self>,
        updated_text: String,
    ) -> bool {
        this.processor.set_modifiable_text(updated_text.as_str())
    }

    pub fn get_attribute(
        #[this] this: &mut ZendClassObject<Self>,
        prefix: BinarySlice<u8>,
    ) -> Option<AttributeValueWrapper> {
        this.processor
            .get_attribute(&prefix)
            .map(|val| AttributeValueWrapper(val))
    }

    pub fn get_attribute_names_with_prefix(
        #[this] this: &mut ZendClassObject<Self>,
        prefix: BinarySlice<u8>,
    ) -> Option<Vec<Binary<u8>>> {
        this.processor
            .get_attribute_names_with_prefix(&prefix)
            .map(|vec| {
                vec.iter()
                    .map(|&name| name.to_vec().into())
                    .collect::<Vec<Binary<u8>>>()
            })
    }

    pub fn paused_at_incomplete_token(#[this] this: &ZendClassObject<Self>) -> bool {
        this.processor.paused_at_incomplete_token()
    }

    pub fn get_full_comment_text(#[this] this: &ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor
            .get_full_comment_text()
            .map(|value| value.to_vec().into())
    }
}

struct AttributeValueWrapper(AttributeValue);
impl IntoZval for AttributeValueWrapper {
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

#[php_class]
pub struct WP_HTML_Processor {
    processor: HtmlProcessor,
}

#[php_impl(rename_methods = "none")]
impl WP_HTML_Processor {
    pub fn create_fragment(
        html: &str,
        context: Option<&str>,
        encoding: Option<&str>,
    ) -> Option<Self> {
        let context = context.unwrap_or("<body>");
        let encoding = encoding.unwrap_or("UTF-8");
        HtmlProcessor::create_fragment(html, context, encoding).map(|processor| Self { processor })
    }

    pub fn create_full_parser(
        html: BinarySlice<u8>,
        known_definite_encoding: Option<&str>,
    ) -> Option<Self> {
        let known_definite_encoding = known_definite_encoding.unwrap_or("UTF-8");
        HtmlProcessor::create_full_parser(html.clone(), known_definite_encoding)
            .map(|processor| Self { processor })
    }

    pub fn next_token(#[this] this: &mut ZendClassObject<Self>) -> bool {
        this.processor.next_token()
    }

    pub fn get_tag(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor.get_tag().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_tag_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor.get_qualified_tag_name().map(|tag_name| {
            let tag_name: Box<[u8]> = tag_name.into();
            tag_name.to_vec().into()
        })
    }

    pub fn get_qualified_attribute_name(
        #[this] this: &mut ZendClassObject<Self>,
        attribute_name: BinarySlice<u8>,
    ) -> Option<Binary<u8>> {
        this.processor
            .get_qualified_attribute_name(&attribute_name)
            .map(|tag_name| {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            })
    }

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        let get_token_name = this.processor.get_token_name()?;
        Some(match get_token_name {
            NodeName::Tag(tag_name) => {
                let tag_name: Box<[u8]> = tag_name.into();
                tag_name.to_vec().into()
            }
            NodeName::Token(token_name) => {
                let token_name: String = token_name.into();
                token_name.as_bytes().to_vec().into()
            }
        })
    }

    pub fn get_attribute_names_with_prefix(
        #[this] this: &mut ZendClassObject<Self>,
        prefix: BinarySlice<u8>,
    ) -> Option<Vec<Binary<u8>>> {
        this.processor
            .get_attribute_names_with_prefix(&prefix)
            .map(|vec| {
                vec.iter()
                    .map(|&name| name.to_vec().into())
                    .collect::<Vec<Binary<u8>>>()
            })
    }

    pub fn get_attribute(
        #[this] this: &mut ZendClassObject<Self>,
        prefix: BinarySlice<u8>,
    ) -> Option<AttributeValueWrapper> {
        this.processor
            .get_attribute(&prefix)
            .map(|val| AttributeValueWrapper(val))
    }

    pub fn get_last_error(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_last_error().map(|value| {
            let s: &str = value.into();
            s.to_owned()
        })
    }

    // Fix this to _return_ a nullable value.
    pub fn get_unsupported_exception(#[this] this: &mut ZendClassObject<Self>) -> Result<(), &str> {
        match this.processor.get_unsupported_exception() {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }

    pub fn is_tag_closer(#[this] this: &mut ZendClassObject<Self>) -> bool {
        this.processor.is_tag_closer()
    }

    pub fn get_namespace(#[this] this: &mut ZendClassObject<Self>) -> String {
        this.processor.get_namespace().into()
    }

    pub fn expects_closer(#[this] this: &mut ZendClassObject<Self>) -> Option<bool> {
        this.processor.expects_closer(None)
    }

    pub fn get_modifiable_text(#[this] this: &mut ZendClassObject<Self>) -> Binary<u8> {
        this.processor.get_modifiable_text().to_vec().into()
    }

    pub fn get_doctype_info(
        #[this] this: &mut ZendClassObject<Self>,
    ) -> Option<WP_HTML_Doctype_Info> {
        this.processor
            .get_doctype_info()
            .map(|internal| WP_HTML_Doctype_Info { internal })
    }

    pub fn paused_at_incomplete_token(#[this] this: &ZendClassObject<Self>) -> bool {
        this.processor.paused_at_incomplete_token()
    }

    pub fn get_full_comment_text(#[this] this: &ZendClassObject<Self>) -> Option<Binary<u8>> {
        this.processor
            .get_full_comment_text()
            .map(|value| value.to_vec().into())
    }
}

#[php_class]
pub struct WP_HTML_Doctype_Info {
    internal: HtmlDoctypeInfo,
}

#[php_impl(rename_methods = "none")]
impl WP_HTML_Doctype_Info {
    pub fn from_doctype_token(html: BinarySlice<u8>) -> Option<Self> {
        let html = html.to_vec();
        HtmlDoctypeInfo::from_doctype_token(&html).map(|internal| Self { internal })
    }

    #[getter]
    pub fn name(&self) -> Option<Binary<u8>> {
        self.internal.name.as_ref().map(|val| val.to_vec().into())
    }

    #[getter]
    pub fn public_identifier(&self) -> Option<Binary<u8>> {
        self.internal
            .public_identifier
            .as_ref()
            .map(|val| val.to_vec().into())
    }

    #[getter]
    pub fn system_identifier(&self) -> Option<Binary<u8>> {
        self.internal
            .system_identifier
            .as_ref()
            .map(|val| val.to_vec().into())
    }

    #[getter]
    pub fn indicated_compatability_mode(&self) -> String {
        (&self.internal.indicated_compatability_mode).into()
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        .request_startup_function(request_startup)
        .request_shutdown_function(request_shutdown);
    module
}
