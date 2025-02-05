#![allow(non_camel_case_types)]

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
        self.processor.get_tag().map(tag_name_to_binary)
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
        self.processor.get_tag().map(tag_name_to_binary)
    }

    pub fn get_token_type(&self) -> Option<String> {
        self.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(&self) -> Option<Box<[u8]>> {
        todo!();
    }
}

fn tag_name_to_binary(tag_name: TagName) -> Box<[u8]> {
    match tag_name {
        TagName::A => b"A".to_vec(),
        TagName::ADDRESS => b"ADDRESS".to_vec(),
        TagName::APPLET => b"APPLET".to_vec(),
        TagName::AREA => b"AREA".to_vec(),
        TagName::ARTICLE => b"ARTICLE".to_vec(),
        TagName::ASIDE => b"ASIDE".to_vec(),
        TagName::B => b"B".to_vec(),
        TagName::BASE => b"BASE".to_vec(),
        TagName::BASEFONT => b"BASEFONT".to_vec(),
        TagName::BGSOUND => b"BGSOUND".to_vec(),
        TagName::BIG => b"BIG".to_vec(),
        TagName::BLOCKQUOTE => b"BLOCKQUOTE".to_vec(),
        TagName::BODY => b"BODY".to_vec(),
        TagName::BR => b"BR".to_vec(),
        TagName::BUTTON => b"BUTTON".to_vec(),
        TagName::CAPTION => b"CAPTION".to_vec(),
        TagName::CENTER => b"CENTER".to_vec(),
        TagName::CODE => b"CODE".to_vec(),
        TagName::COL => b"COL".to_vec(),
        TagName::COLGROUP => b"COLGROUP".to_vec(),
        TagName::DD => b"DD".to_vec(),
        TagName::DETAILS => b"DETAILS".to_vec(),
        TagName::DIALOG => b"DIALOG".to_vec(),
        TagName::DIR => b"DIR".to_vec(),
        TagName::DIV => b"DIV".to_vec(),
        TagName::DL => b"DL".to_vec(),
        TagName::DT => b"DT".to_vec(),
        TagName::EM => b"EM".to_vec(),
        TagName::EMBED => b"EMBED".to_vec(),
        TagName::FIELDSET => b"FIELDSET".to_vec(),
        TagName::FIGCAPTION => b"FIGCAPTION".to_vec(),
        TagName::FIGURE => b"FIGURE".to_vec(),
        TagName::FONT => b"FONT".to_vec(),
        TagName::FOOTER => b"FOOTER".to_vec(),
        TagName::FORM => b"FORM".to_vec(),
        TagName::FRAME => b"FRAME".to_vec(),
        TagName::FRAMESET => b"FRAMESET".to_vec(),
        TagName::HEAD => b"HEAD".to_vec(),
        TagName::HEADER => b"HEADER".to_vec(),
        TagName::HGROUP => b"HGROUP".to_vec(),
        TagName::HR => b"HR".to_vec(),
        TagName::HTML => b"HTML".to_vec(),
        TagName::I => b"I".to_vec(),
        TagName::IFRAME => b"IFRAME".to_vec(),
        TagName::IMG => b"IMG".to_vec(),
        TagName::INPUT => b"INPUT".to_vec(),
        TagName::KEYGEN => b"KEYGEN".to_vec(),
        TagName::LI => b"LI".to_vec(),
        TagName::LINK => b"LINK".to_vec(),
        TagName::LISTING => b"LISTING".to_vec(),
        TagName::MAIN => b"MAIN".to_vec(),
        TagName::MARQUEE => b"MARQUEE".to_vec(),
        TagName::MATH => b"MATH".to_vec(),
        TagName::MENU => b"MENU".to_vec(),
        TagName::META => b"META".to_vec(),
        TagName::NAV => b"NAV".to_vec(),
        TagName::NOBR => b"NOBR".to_vec(),
        TagName::NOEMBED => b"NOEMBED".to_vec(),
        TagName::NOFRAMES => b"NOFRAMES".to_vec(),
        TagName::NOSCRIPT => b"NOSCRIPT".to_vec(),
        TagName::OBJECT => b"OBJECT".to_vec(),
        TagName::OL => b"OL".to_vec(),
        TagName::OPTGROUP => b"OPTGROUP".to_vec(),
        TagName::OPTION => b"OPTION".to_vec(),
        TagName::P => b"P".to_vec(),
        TagName::PARAM => b"PARAM".to_vec(),
        TagName::PLAINTEXT => b"PLAINTEXT".to_vec(),
        TagName::PRE => b"PRE".to_vec(),
        TagName::RB => b"RB".to_vec(),
        TagName::RP => b"RP".to_vec(),
        TagName::RT => b"RT".to_vec(),
        TagName::RTC => b"RTC".to_vec(),
        TagName::RUBY => b"RUBY".to_vec(),
        TagName::S => b"S".to_vec(),
        TagName::SCRIPT => b"SCRIPT".to_vec(),
        TagName::SEARCH => b"SEARCH".to_vec(),
        TagName::SECTION => b"SECTION".to_vec(),
        TagName::SELECT => b"SELECT".to_vec(),
        TagName::SMALL => b"SMALL".to_vec(),
        TagName::SOURCE => b"SOURCE".to_vec(),
        TagName::SPAN => b"SPAN".to_vec(),
        TagName::STRIKE => b"STRIKE".to_vec(),
        TagName::STRONG => b"STRONG".to_vec(),
        TagName::STYLE => b"STYLE".to_vec(),
        TagName::SUB => b"SUB".to_vec(),
        TagName::SUMMARY => b"SUMMARY".to_vec(),
        TagName::SUP => b"SUP".to_vec(),
        TagName::SVG => b"SVG".to_vec(),
        TagName::TABLE => b"TABLE".to_vec(),
        TagName::TBODY => b"TBODY".to_vec(),
        TagName::TD => b"TD".to_vec(),
        TagName::TEMPLATE => b"TEMPLATE".to_vec(),
        TagName::TEXTAREA => b"TEXTAREA".to_vec(),
        TagName::TFOOT => b"TFOOT".to_vec(),
        TagName::TH => b"TH".to_vec(),
        TagName::THEAD => b"THEAD".to_vec(),
        TagName::TITLE => b"TITLE".to_vec(),
        TagName::TR => b"TR".to_vec(),
        TagName::TRACK => b"TRACK".to_vec(),
        TagName::TT => b"TT".to_vec(),
        TagName::U => b"U".to_vec(),
        TagName::UL => b"UL".to_vec(),
        TagName::VAR => b"VAR".to_vec(),
        TagName::WBR => b"WBR".to_vec(),
        TagName::XMP => b"XMP".to_vec(),
        TagName::Doctype => b"html".to_vec(),
        TagName::Arbitrary(arbitrary_name) => arbitrary_name.to_vec(),
    }
    .into()
}
