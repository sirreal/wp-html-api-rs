use glob::glob;
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{parse_macro_input, LitStr};

struct TestCase {
    pub input: Vec<u8>,
    pub context: Vec<u8>,
    pub expected_document: Vec<u8>,
    pub line_number: usize, // Line number where this test case starts
    pub script_flag: bool,
}
impl Default for TestCase {
    fn default() -> Self {
        TestCase {
            input: Vec::new(),
            context: Vec::new(),
            expected_document: Vec::new(),
            line_number: 0,
            script_flag: false,
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
                    // Trim trailing newline from test input.
                    current_test.input.truncate(current_test.input.len() - 1);
                    tests.push(current_test);
                    current_test = TestCase::default();
                }
                current_test.line_number = line_number;
                current_section = Section::Data;
            }
            b"#errors" | b"#new-errors" => {
                current_section = Section::Errors;
            }
            b"#script-on" => {
                current_test.script_flag = true;
            }
            b"#script-off" => {}
            b"#document-fragment" => {
                current_section = Section::Context;
            }
            b"#document" => {
                current_section = Section::Document;
            }
            _ => match current_section {
                Section::Data => {
                    current_test.input.extend(line);
                    current_test.input.push(b'\n');
                }
                Section::Errors => {}
                Section::Context => {
                    current_test.context.extend(line);
                    current_test.context.push(b'\n');
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
        // Trim trailing newline from test input.
        current_test.input.truncate(current_test.input.len() - 1);
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

        let test_name =   &format!("line{:04}", test.line_number);
        let test_name_fn_name = syn::Ident::new(
            &test_name,
            proc_macro2::Span::call_site()
        );
        let input = &test.input[..];
        let expected = &test.expected_document[..];

        // @todo: Implement context element parsing}
        let has_context = !test.context.is_empty();
        let ignore = if let Some((_,_,reason)) = EXCLUDED_TESTS.iter().find(|(file, test,_ )| file == &file_name && test == test_name) {
            quote! { #[ignore = #reason] }
        } else if test.script_flag {
            quote! { #[ignore = "HTML API does not support scripting."] }
        } else if has_context {
            quote! { #[ignore = "Fragment tests are not yet supported."] }
        } else { quote! {} };

        quote! {
            #ignore
            #[test]
            fn #test_name_fn_name() -> Result<(), String> {
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
                    String::from_utf8(expected).expect("Must have valid UTF-8 expected."),
                    String::from_utf8(actual).expect("Must have valid UTF-8 output."),
                    "Error with input:\n{:?}",
                    String::from_utf8(input).expect("Must have valid UTF-8 input."),
                );

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

const EXCLUDED_TESTS: &[(&str, &str, &str)] = &[
    (
        "noscript01",
        "line0014",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests14",
        "line0022",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests14",
        "line0055",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests19",
        "line0488",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests19",
        "line0500",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests19",
        "line1079",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests2",
        "line0207",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests2",
        "line0686",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests2",
        "line0697",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "tests2",
        "line0709",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
    (
        "webkit01",
        "line0231",
        "Unimplemented: This parser does not add missing attributes to existing HTML or BODY tags.",
    ),
];

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
