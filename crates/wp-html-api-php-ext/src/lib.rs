#![allow(non_camel_case_types)]
use ext_php_rs::{
    binary::Binary, binary_slice::BinarySlice, builders::ModuleBuilder, prelude::*,
    types::ZendClassObject,
};
use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::{NodeName, TagProcessor};

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

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] _this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        todo!();
    }

    pub fn get_modifiable_text(#[this] this: &mut ZendClassObject<Self>) -> String {
        this.processor.get_modifiable_text().as_ref().into()
    }

    pub fn set_modifiable_text(
        #[this] this: &mut ZendClassObject<Self>,
        updated_text: String,
    ) -> bool {
        this.processor.set_modifiable_text(updated_text.as_str())
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
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        .request_startup_function(request_startup)
        .request_shutdown_function(request_shutdown);
    module
}
