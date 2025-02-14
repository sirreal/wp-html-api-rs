#![allow(unused_imports)]

use std::fs;

use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::TagProcessor;

pub fn main() {
    let html = fs::read_to_string("./html-standard.html").expect("Missing input!");

    let mut p = TagProcessor::new(html.as_bytes());

    let mut count_tagp = 0u32;
    while p.next_token() {
        count_tagp += 1;
    }

    println!(
        "No more tokens found in TagProcessor. Bytes parsed: {}",
        p.bytes_already_parsed
    );

    let mut p = HtmlProcessor::create_full_parser(html.as_bytes(), "UTF-8").unwrap();

    let mut count_htmlp = 0u32;
    while p.next_token() {
        //match p.get_token_name() {
        //    Some(node) => match node {
        //        wp_html_api::tag_processor::NodeName::Tag(tag_name) => {
        //            let closer = if p.is_tag_closer() { "/" } else { "" };
        //            println!("{closer}{} // token number {}", tag_name, c);
        //        }
        //        wp_html_api::tag_processor::NodeName::Token(token_type) => {
        //            let s: String = token_type.into();
        //            println!("{}  // token number {}", s, c);
        //        }
        //    },
        //    None => {}
        //};

        count_htmlp += 1;
    }

    println!(
        "No more tokens found in HtmlProcessor. Bytes parsed: {}",
        p.tag_processor.bytes_already_parsed
    );
    //println!(
    //    "Stopped around:\n{}",
    //    String::from_utf8_lossy(
    //        &p.tag_processor.html_bytes[p.tag_processor.bytes_already_parsed - 100
    //            ..=p.tag_processor.bytes_already_parsed + 100]
    //    )
    //);
    //println!(
    //    "Specifically:\n{}",
    //    String::from_utf8_lossy(
    //        &p.tag_processor.html_bytes[p.tag_processor.bytes_already_parsed - 4
    //            ..=p.tag_processor.bytes_already_parsed + 4]
    //    )
    //);

    println!("Tag processor found {count_tagp} tokens!");
    println!("Html processor found {count_htmlp} tokens!");
    if count_tagp != count_htmlp {
        println!("Tag processor and Html processor found different number of tokens!");
    }
}
