use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{parse_macro_input, LitStr};

struct TestCase {
    input: String,
    errors: Vec<(usize, usize, String)>, // (line, col, message)
    expected_document: String,
}

fn parse_test_file(content: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();
    let mut current_section = None;
    let mut current_test = TestCase {
        input: String::new(),
        errors: Vec::new(),
        expected_document: String::new(),
    };

    for line in content.lines() {
        if line.starts_with("#data") {
            if !current_test.input.is_empty() {
                tests.push(current_test);
                current_test = TestCase {
                    input: String::new(),
                    errors: Vec::new(),
                    expected_document: String::new(),
                };
            }
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

#[proc_macro]
pub fn html5lib_tests(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let test_file_path = input.value();

    let content = fs::read_to_string(test_file_path).expect("Failed to read test file");

    let test_cases = parse_test_file(&content);

    let test_fns = test_cases.iter().enumerate().map(|(i, test)| {
        let test_name = syn::Ident::new(&format!("html5lib_test_{}", i), proc_macro2::Span::call_site());
        let input = &test.input;
        let expected = &test.expected_document;

        // Generate error assertions
        let error_assertions = test.errors.iter().map(|(line, col, msg)| {
            quote! {
                assert_error(&processor, #line, #col, #msg);
            }
        });

        quote! {
            #[test]
            fn #test_name () {
                let input = #input;
                let expected = #expected;

                let mut processor = HtmlProcessor::create_full_parser(input.as_bytes(), "UTF-8").expect("Failed to create HTML processor");
                while processor.next_token() {}

                // TODO: Add actual assertions once we have tree comparison logic
                // For now we'll just print the input and expected output
                println!("Input:\n{}", input);
                println!("Expected:\n{}", expected);

                #(#error_assertions)*
            }
        }
    });

    let expanded = quote! {
        mod html5lib_tests_generated {
            use wp_html_api::html_processor::{HtmlProcessor, errors::HtmlProcessorError};

            fn assert_error(processor: &HtmlProcessor, line: usize, col: usize, expected_msg: &str) {
                // TODO: Once error reporting is implemented in HtmlProcessor,
                // this will check if the processor has recorded the expected error
                // For now we just print the expected error
                println!("Expected error at {}:{}: {}", line, col, expected_msg);
            }

            #(#test_fns)*
        }
    };

    expanded.into()
}
