#![allow(unused_imports)]

mod tag_processor;
use std::fs;
use tag_processor::TagProcessor;
mod html_processor;
use html_processor::HtmlProcessor;

pub fn main() {
    let html = fs::read_to_string("./html-standard.html").expect("Missing input!");
    let mut c = 0u32;

    let mut p = HtmlProcessor::create_full_parser(html.as_bytes(), "UTF-8").unwrap();

    while p.next_token() {
        c += 1;
        let closer = if p.is_tag_closer() { "/" } else { "" };
        println!("{closer}{:?}", p.get_token_name());
    }
    if p.get_last_error().is_some() {
        println!("{:?}", p.get_last_error());
    }

    println!("Found {c} tokens!");
}
