#![allow(non_camel_case_types, unused_macros)]

extern crate wasm_bindgen;
use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::{AttributeValue, NodeName, TagProcessor, TokenType};

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
    pub fn new(html: String) -> Self {
        let processor = TagProcessor::new(html.as_bytes());
        Self { processor }
    }

    pub fn is_tag_closer(&self) -> bool {
        self.processor.is_tag_closer()
    }

    pub fn get_updated_html(&self) -> () {
        todo!()
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn get_tag(&self) -> Option<Box<[u8]>> {
        self.processor.get_tag().map(Into::into)
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(Into::into)
    }

    pub fn get_token_name(&self) -> Option<Box<[u8]>> {
        self.processor.get_token_name().map(|name| match name {
            NodeName::Tag(tag_name) => tag_name.into(),
            NodeName::Token(token_name) => {
                if token_name == TokenType::Doctype {
                    b"html".as_ref().into()
                } else {
                    let token_name: &str = (&token_name).into();
                    token_name.as_bytes().into()
                }
            }
        })
    }

    pub fn get_modifiable_text(&self) -> Box<[u8]> {
        self.processor.get_modifiable_text()
    }
}

#[wasm_bindgen]
pub struct WP_HTML_Processor {
    processor: HtmlProcessor,
}

#[wasm_bindgen]
impl WP_HTML_Processor {
    pub fn create_full_parser(
        html: String,
        known_definite_encoding: Option<String>,
    ) -> Option<Self> {
        let known_definite_encoding = known_definite_encoding.unwrap_or("UTF-8".to_owned());
        HtmlProcessor::create_full_parser(html.as_bytes(), &known_definite_encoding)
            .map(|processor| Self { processor })
    }

    pub fn is_tag_closer(&self) -> bool {
        self.processor.is_tag_closer()
    }

    pub fn next_token(&mut self) -> bool {
        self.processor.next_token()
    }

    pub fn get_tag(&self) -> Option<Box<[u8]>> {
        self.processor.get_tag().map(Into::into)
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(Into::into)
    }

    pub fn get_token_name(&self) -> Option<Box<[u8]>> {
        self.processor.get_token_name().map(|name| match name {
            NodeName::Tag(tag_name) => tag_name.into(),
            NodeName::Token(token_name) => {
                if token_name == TokenType::Doctype {
                    b"html".as_ref().into()
                } else {
                    let token_name: &str = (&token_name).into();
                    token_name.as_bytes().into()
                }
            }
        })
    }

    pub fn get_attribute(&self, prefix: String) -> JsValue {
        self.processor.get_attribute(prefix.as_bytes()).map_or_else(
            || JsValue::null(),
            |val| match val {
                AttributeValue::BooleanFalse => JsValue::null(),
                AttributeValue::BooleanTrue => JsValue::TRUE,
                AttributeValue::String(s) => s.into(),
            },
        )
    }

    pub fn class_list(&self) -> Box<[String]> {
        let vec: Vec<String> = self
            .processor
            .class_list()
            .into_iter()
            .map(|s| -> String { String::from_utf8_lossy(&s.to_vec()).into() })
            .collect();
        vec.into_boxed_slice()
    }

    pub fn get_modifiable_text(&self) -> Box<[u8]> {
        self.processor.get_modifiable_text()
    }

    pub fn get_last_error(&self) -> Option<String> {
        self.processor.get_last_error().map(|value| {
            let s: &str = value.into();
            s.to_owned()
        })
    }

    pub fn get_breadcrumbs(&self) -> Box<[String]> {
        self.processor
            .get_breadcrumbs()
            .into_iter()
            .map(|name| match name {
                NodeName::Tag(tag_name) => {
                    let s: Box<[u8]> = tag_name.into();
                    String::from_utf8_lossy(&s.to_vec()).into()
                }
                NodeName::Token(token_name) => {
                    if token_name == &TokenType::Doctype {
                        "html".to_owned()
                    } else {
                        let token_name: &str = token_name.into();
                        token_name.to_owned()
                    }
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}
