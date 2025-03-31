use html_to_md::HtmlToMarkdown;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    // Read HTML bytes from stdin
    let mut html_bytes = Vec::new();
    let html = io::stdin().read_to_end(&mut html_bytes)?;

    let stdout = io::stdout();
    let mut writer = stdout.lock();
    HtmlToMarkdown::convert_to_writer(&html_bytes, &mut writer, "", 80)
}
