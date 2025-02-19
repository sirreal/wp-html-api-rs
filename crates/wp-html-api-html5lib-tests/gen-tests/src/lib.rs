const TREE_INDENT: &str = "    ";

pub struct TestCase {
    pub input: String,
    pub errors: Vec<(usize, usize, String)>, // (line, col, message)
    pub expected_document: String,
    pub line_number: usize, // Line number where this test case starts
}

pub fn parse_test_file(content: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();
    let mut current_section = None;
    let mut current_test = TestCase {
        input: String::new(),
        errors: Vec::new(),
        expected_document: String::new(),
        line_number: 0,
    };
    let mut line_number = 0;

    for line in content.lines() {
        line_number += 1;
        if line.starts_with("#data") {
            if !current_test.input.is_empty() {
                tests.push(current_test);
                current_test = TestCase {
                    input: String::new(),
                    errors: Vec::new(),
                    expected_document: String::new(),
                    line_number: 0,
                };
            }
            current_test.line_number = line_number + 1; // Test data starts on next line
            current_section = Some("data");
        } else if line.starts_with("#errors") {
            current_section = Some("errors");
        } else if line.starts_with("#document") {
            current_section = Some("document");
        } else {
            match current_section {
                Some("data") => {
                    current_test.input.push_str(line);
                    current_test.input.push('\n');
                }
                Some("errors") => {
                    if !line.is_empty() {
                        // Parse error line like "(1,0): expected-doctype-but-got-chars"
                        let parts: Vec<_> = line.split(": ").collect();
                        if parts.len() == 2 {
                            let coords = parts[0].trim_matches(|c| c == '(' || c == ')');
                            let message = parts[1];
                            if let Some((line, col)) = coords.split_once(',') {
                                if let (Ok(line), Ok(col)) = (line.parse(), col.parse()) {
                                    current_test.errors.push((line, col, message.to_string()));
                                }
                            }
                        }
                    }
                }
                Some("document") => {
                    current_test.expected_document.push_str(line);
                    current_test.expected_document.push('\n');
                }
                _ => {}
            }
        }
    }

    if !current_test.input.is_empty() {
        tests.push(current_test);
    }

    tests
}

pub fn build_tree_representation(
    processor: &mut wp_html_api::html_processor::HtmlProcessor,
) -> String {
    let mut output = String::new();
    let mut indent_level = 0;
    let mut was_text = false;
    let mut text_node = String::new();

    while processor.next_token() {
        let token_type = processor.get_token_type();
        let is_closer = processor.is_tag_closer();

        // Handle text node buffering
        if was_text && token_type != Some(&wp_html_api::tag_processor::TokenType::Text) {
            if !text_node.is_empty() {
                output.push_str(&text_node);
                output.push_str("\"\n");
            }
            was_text = false;
            text_node.clear();
        }

        match token_type {
            Some(&wp_html_api::tag_processor::TokenType::Doctype) => {
                if let Some(doctype) = processor.get_doctype_info() {
                    output.push_str("<!DOCTYPE ");
                    if let Some(name) = doctype.name {
                        output.push_str(&String::from_utf8_lossy(&name));
                    }
                    if doctype.public_identifier.is_some() || doctype.system_identifier.is_some() {
                        if let Some(public_id) = doctype.public_identifier {
                            output
                                .push_str(&format!(" \"{}\"", String::from_utf8_lossy(&public_id)));
                        }
                        if let Some(system_id) = doctype.system_identifier {
                            output
                                .push_str(&format!(" \"{}\"", String::from_utf8_lossy(&system_id)));
                        }
                    }
                    output.push_str(">\n");
                }
            }
            Some(&wp_html_api::tag_processor::TokenType::Tag) => {
                let namespace = "html"; // TODO: Get actual namespace when implemented
                let tag_name = processor.get_tag().unwrap();
                let tag_bytes: Box<[u8]> = tag_name.clone().into();
                let tag_name = if namespace == "html" {
                    String::from_utf8_lossy(&tag_bytes).to_lowercase()
                } else {
                    format!("{} {}", namespace, String::from_utf8_lossy(&tag_bytes))
                };

                if is_closer {
                    indent_level -= 1;
                    if namespace == "html" && tag_name.eq_ignore_ascii_case("template") {
                        indent_level -= 1;
                    }
                    continue;
                }

                let tag_indent = indent_level;
                if processor.expects_closer(None).unwrap_or(false) {
                    indent_level += 1;
                }

                // Write tag
                output.push_str(&TREE_INDENT.repeat(tag_indent));
                output.push_str(&format!("<{}>\n", tag_name));

                // TODO: Handle attributes when API is available

                // Handle template content
                if namespace == "html" && tag_name.eq_ignore_ascii_case("template") {
                    output.push_str(&TREE_INDENT.repeat(indent_level));
                    output.push_str("content\n");
                    indent_level += 1;
                }
            }
            Some(&wp_html_api::tag_processor::TokenType::Text) => {
                let text_content = processor.get_modifiable_text();
                if text_content.is_empty() {
                    continue;
                }

                was_text = true;
                if text_node.is_empty() {
                    text_node = TREE_INDENT.repeat(indent_level);
                    text_node.push('"');
                }
                text_node.push_str(&String::from_utf8_lossy(&text_content));
            }
            Some(&wp_html_api::tag_processor::TokenType::Comment) => {
                let comment = processor.get_full_comment_text().unwrap();
                output.push_str(&TREE_INDENT.repeat(indent_level));
                output.push_str(&format!("<!-- {} -->\n", String::from_utf8_lossy(&comment)));
            }
            _ => {}
        }
    }

    // Handle any remaining text node
    if !text_node.is_empty() {
        output.push_str(&text_node);
        output.push_str("\"\n");
    }

    // Tests always end with a trailing newline
    output.push('\n');
    output
}
