#![allow(non_camel_case_types, unused_macros)]

extern crate wasm_bindgen;
use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::{TagName, TagProcessor};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct WP_HTML_Tag_Processor {
    processor: TagProcessor,
}

#[wasm_bindgen]
impl WP_HTML_Tag_Processor {
    #[wasm_bindgen(constructor)]
    pub fn new(html: &[u8]) -> Self {
        let processor = TagProcessor::new(html);
        Self { processor }
    }

    pub fn get_updated_html(&self) -> () {
        todo!()
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn get_tag(&self) -> Option<Box<[u8]>> {
        self.processor.get_tag().map(TagName::into)
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(&self) -> Option<Box<[u8]>> {
        todo!();
    }

    pub fn get_modifiable_text(&self) -> () {
        todo!()
    }
}

#[wasm_bindgen]
pub struct WP_HTML_Processor {
    processor: HtmlProcessor,
}

#[wasm_bindgen]
impl WP_HTML_Processor {
    pub fn create_full_parser(
        html: Box<[u8]>,
        known_definite_encoding: Option<Box<[u8]>>,
    ) -> Option<Self> {
        let known_definite_encoding = known_definite_encoding
            .and_then(|val| String::from_utf8(val.to_vec()).ok())
            .unwrap_or("UTF-8".to_owned());
        HtmlProcessor::create_full_parser(html.as_ref(), &known_definite_encoding)
            .map(|processor| Self { processor })
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn get_tag(&self) -> Option<Box<[u8]>> {
        self.processor.get_tag().map(TagName::into)
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(&self) -> Option<Box<[u8]>> {
        todo!();
    }
}
