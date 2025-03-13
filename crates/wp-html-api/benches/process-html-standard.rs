use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::TagProcessor;

const INPUT: &[u8] = include_bytes!("../../../data/html-standard.html");

fn main() {
    divan::main();
}

#[divan::bench(skip_ext_time = true)]
fn bench_html_processor(bencher: divan::Bencher) {
    bencher.bench(|| {
        let mut processor =
            HtmlProcessor::create_full_parser(INPUT, "UTF-8").expect("Processor must read input");
        while processor.next_token() {}
        processor
    });
}

#[divan::bench(skip_ext_time = true)]
fn bench_tag_processor(bencher: divan::Bencher) {
    bencher.bench(|| {
        let mut processor = TagProcessor::new(INPUT);
        while processor.next_token() {}
        processor
    });
}
