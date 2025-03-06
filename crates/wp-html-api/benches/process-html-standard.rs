use wp_html_api::html_processor::HtmlProcessor;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_html_processor_parse_spec(bencher: divan::Bencher) {
    let input = std::fs::read("../../data/html-standard.html").expect("Missing input!");

    bencher.bench(|| {
        let mut processor =
            HtmlProcessor::create_full_parser(&input, "UTF-8").expect("Processor must read input");
        while processor.next_token() {}
    });
}
