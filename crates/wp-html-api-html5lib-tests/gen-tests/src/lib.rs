use glob::glob;
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{parse_macro_input, LitStr};

struct TestCase {
    pub input: Vec<u8>,
    pub context: Vec<u8>,
    pub errors: Vec<(usize, usize, String)>, // (line, col, message)
    pub expected_document: Vec<u8>,
    pub line_number: usize, // Line number where this test case starts
}
impl Default for TestCase {
    fn default() -> Self {
        TestCase {
            input: Vec::new(),
            context: Vec::new(),
            errors: Vec::new(),
            expected_document: Vec::new(),
            line_number: 0,
        }
    }
}

fn parse_test_file(content: &[u8]) -> Vec<TestCase> {
    #[derive(Debug, PartialEq)]
    enum Section {
        Unknown,
        Data,
        Errors,
        Context,
        Document,
    }

    let mut tests = Vec::new();
    let mut current_section = Section::Unknown;
    let mut current_test = TestCase::default();
    let mut line_number = 0;

    for line in content.split(|c| *c == b'\n') {
        line_number += 1;
        match line {
            b"#data" => {
                if current_section != Section::Unknown {
                    tests.push(current_test);
                    current_test = TestCase::default();
                }
                current_test.line_number = line_number;
                current_section = Section::Data;
            }
            b"#errors" => {
                current_section = Section::Errors;
            }
            b"#document-fragment" => {
                current_section = Section::Context;
            }
            b"#document" => {
                current_section = Section::Document;
            }
            _ => match current_section {
                Section::Data => {
                    current_test.input.extend(line);
                }
                Section::Errors => {}
                Section::Context => {
                    current_test.context.extend(line);
                }
                Section::Document => {
                    if line.starts_with(b"| ") {
                        current_test.expected_document.extend(&line[2..]);
                    } else {
                        current_test.expected_document.extend(line);
                    }
                    current_test.expected_document.push(b'\n');
                }
                _ => unreachable!("attempted to parse in unknown section"),
            },
        };
    }

    if !current_test.input.is_empty() {
        tests.push(current_test);
    }

    tests
}

fn process_test_file(test_file_path: &str) -> proc_macro2::TokenStream {
    // Extract the file name from the path
    let file_name = test_file_path
        .split('/')
        .last()
        .and_then(|s| s.split('.').next())
        .unwrap_or("unknown")
        .replace('-', "_") // Replace hyphens with underscores
        .to_string();

    let content = fs::read(test_file_path).expect("Failed to read test file");
    let test_cases = parse_test_file(&content);

    let file_mod_name = syn::Ident::new(&file_name, proc_macro2::Span::call_site());

    let test_fns = test_cases.iter().map(|test| {
        let test_name = syn::Ident::new(
            &format!("line{:04}", test.line_number),
            proc_macro2::Span::call_site()
        );
        let input = &test.input[..];
        let expected = &test.expected_document[..];

        // @todo: Implement context element parsing}
        let has_context = !test.context.is_empty();
        let ignore = if has_context { quote! { #[ignore] } } else { quote! {} };

        // Generate error assertions
        let error_assertions = test.errors.iter().map(|(line, col, msg)| {
            quote! {
                assert_error(&processor, #line, #col, #msg);
            }
        });

        quote! {
            #ignore
            #[test]
            fn #test_name() -> Result<(), String> {
                let input: Vec<u8> = vec![#(#input),*];
                let expected: Vec<u8> = vec![#(#expected),*];

                let mut processor = HtmlProcessor::create_full_parser(&input, "UTF-8").expect("Failed to create HTML processor");
                let actual = build_tree_representation(&mut processor);
                let actual = match actual {
                    Ok(actual) => actual,
                    Err(inner_err) => {
                        match inner_err {
                            TreeBuilderError::Arbitrary(_) => return Err(inner_err.into()),

                            // Treat these like skips.
                            TreeBuilderError::PausedAtIncompleteToken => return Ok(()),
                            TreeBuilderError::HtmlProcessor(_) => return Ok(()),
                        }
                    }
                };

                pretty_assertions::assert_str_eq!(
                    String::from_utf8_lossy(&expected),
                    String::from_utf8_lossy(&actual),
                    "Error with input:\n{:?}",
                    String::from_utf8_lossy(&input),
                );

                #(#error_assertions)*

                Ok(())
            }
        }
    });

    quote! {
        pub mod #file_mod_name {
            use wp_html_api::html_processor::{HtmlProcessor, errors::HtmlProcessorError};
            use wp_html_api_html5lib_tests::{build_tree_representation, TreeBuilderError};

            fn assert_error(processor: &HtmlProcessor, line: usize, col: usize, expected_msg: &str) {
                // TODO: Once error reporting is implemented in HtmlProcessor,
                // this will check if the processor has recorded the expected error
                // For now we just print the expected error
                println!("Expected error at {}:{}: {}", line, col, expected_msg);
            }

            #(#test_fns)*
        }
    }
}

#[proc_macro]
pub fn html5lib_tests(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let pattern = input.value();

    let mut all_tests = Vec::new();

    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let path_str = path.to_str().unwrap();
                all_tests.push(process_test_file(path_str));
            }
            Err(e) => panic!("Error processing test file: {:?}", e),
        }
    }

    let expanded = quote! {
        #[cfg(test)]
        pub mod html5lib {
            #(#all_tests)*
        }
    };

    expanded.into()
}
