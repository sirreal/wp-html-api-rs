#![allow(non_camel_case_types)]
mod tag_processor;
use ext_php_rs::{builders::ModuleBuilder, prelude::*, types::ZendClassObject};

extern "C" fn request_startup(_ty: i32, _module_number: i32) -> i32 {
    0
}

extern "C" fn request_shutdown(_ty: i32, _module_number: i32) -> i32 {
    0
}

#[php_class]
pub struct WP_HTML_Tag_Processor {
    processor: html_api::HtmlProcessor,
}

#[php_impl(rename_methods = "none")]
impl WP_HTML_Tag_Processor {
    pub fn __construct(html: &str) -> Self {
        Self::create_fragment(html)
    }

    pub fn create_fragment(html: &str) -> Self {
        let processor = html_api::HtmlProcessor::create_fragment(html);
        Self { processor }
    }

    pub fn next_token(#[this] this: &mut ZendClassObject<Self>) -> bool {
        this.processor.next_token()
    }

    pub fn get_tag(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_tag().map(|t| t.into())
    }

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_name().map(|t| t.into())
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        .request_startup_function(request_startup)
        .request_shutdown_function(request_shutdown);
    module
}
