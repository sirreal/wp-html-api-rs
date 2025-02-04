mod tag_processor;
use std::fs;
use tag_processor::TagProcessor;

pub fn main() {
    let html = fs::read_to_string("./html-standard.html").expect("Missing input!");
    let mut c = 0;

    for _ in 1..100 {
        let mut p = TagProcessor::new(html.as_bytes());

        while p.next_token() {
            c += 1;
        }
    }

    println!("Found {c} tokens!");
}
