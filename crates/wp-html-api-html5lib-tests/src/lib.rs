use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{parse_macro_input, LitStr};
use wp_html_api_html5lib_tests_gen_tests::parse_test_file;

#[proc_macro]
pub fn html5lib_tests(input: TokenStream) -> TokenStream {
    #[allow(unused_imports)]
    use wp_html_api_html5lib_tests_gen_tests::build_tree_representation;

    let input = parse_macro_input!(input as LitStr);
    let test_file_path = input.value();

    // Extract the file name from the path
    let file_name = test_file_path
        .split('/')
        .last()
        .and_then(|s| s.split('.').next())
        .unwrap_or("unknown")
        .to_string();

    let content = fs::read_to_string(&test_file_path).expect("Failed to read test file");

    let test_cases = parse_test_file(&content);

    let file_mod_name = syn::Ident::new(&file_name, proc_macro2::Span::call_site());

    let test_fns = test_cases.iter().map(|test| {
        let test_name = syn::Ident::new(
            &format!("line{:04}", test.line_number),
            proc_macro2::Span::call_site()
        );
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
                let actual = build_tree_representation(&mut processor);

                assert_eq!(actual, expected, "\nExpected:\n{}\nActual:\n{}", expected, actual);

                #(#error_assertions)*
            }
        }
    });

    let expanded = quote! {
        pub mod html5lib_tests {
            pub mod #file_mod_name {
                use super::super::*;
                use wp_html_api::html_processor::{HtmlProcessor, errors::HtmlProcessorError};
                use wp_html_api_html5lib_tests_gen_tests::build_tree_representation;

                fn assert_error(processor: &HtmlProcessor, line: usize, col: usize, expected_msg: &str) {
                    // TODO: Once error reporting is implemented in HtmlProcessor,
                    // this will check if the processor has recorded the expected error
                    // For now we just print the expected error
                    println!("Expected error at {}:{}: {}", line, col, expected_msg);
                }

                #(#test_fns)*
            }
        }
    };

    expanded.into()
}
