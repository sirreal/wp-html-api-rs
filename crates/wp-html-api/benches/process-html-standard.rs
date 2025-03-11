use wp_html_api::html_processor::HtmlProcessor;
use wp_html_api::tag_processor::TagProcessor;

fn main() {
    divan::main();
}

#[divan::bench(skip_ext_time = true)]
fn bench_html_processor(bencher: divan::Bencher) {
    bencher.with_inputs(get_input).bench_values(|input| {
        let mut processor =
            HtmlProcessor::create_full_parser(&input, "UTF-8").expect("Processor must read input");
        while processor.next_token() {}
        processor
    });
}

#[divan::bench(skip_ext_time = true)]
fn bench_tag_processor(bencher: divan::Bencher) {
    bencher.with_inputs(get_input).bench_values(|input| {
        let mut processor = TagProcessor::new(&input);
        while processor.next_token() {}
        processor
    });
}

fn get_input() -> Vec<u8> {
    std::fs::read("../../data/html-standard.html").expect("Missing input!")
}
