#![allow(non_camel_case_types)]
use ext_php_rs::{
    binary::Binary, binary_slice::BinarySlice, builders::ModuleBuilder, prelude::*,
    types::ZendClassObject,
};
use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::{TagName, TagProcessor};

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
        this.processor.get_tag().map(tag_name_to_binary)
    }

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
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
        todo!();
    }

    pub fn get_token_type(#[this] this: &mut ZendClassObject<Self>) -> Option<String> {
        this.processor.get_token_type().map(|t| t.into())
    }

    pub fn get_token_name(#[this] this: &mut ZendClassObject<Self>) -> Option<Binary<u8>> {
        todo!();
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        .request_startup_function(request_startup)
        .request_shutdown_function(request_shutdown);
    module
}

fn tag_name_to_binary(tag_name: TagName) -> Binary<u8> {
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
        TagName::H1 => b"H1".to_vec(),
        TagName::H2 => b"H2".to_vec(),
        TagName::H3 => b"H3".to_vec(),
        TagName::H4 => b"H4".to_vec(),
        TagName::H5 => b"H5".to_vec(),
        TagName::H6 => b"H6".to_vec(),
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
