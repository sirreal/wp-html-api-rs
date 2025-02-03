mod tag_processor;
use tag_processor::TagProcessor;
use std::fs;

pub fn main() {
    let html = fs::read_to_string("/Users/dmsnell/Downloads/single-page.html").expect("Missing input!");
    let mut c = 0;

    for _ in 1..100 {
        let mut p = TagProcessor::new(html.as_bytes());

        while p.next_token() {
            c += 1;
        }
    }

    println!("Found {c} tokens!");
}
