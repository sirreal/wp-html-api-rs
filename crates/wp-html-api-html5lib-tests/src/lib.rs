use glob::glob;
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::{parse_macro_input, LitStr};
use wp_html_api_html5lib_tests_gen_tests::parse_test_file;

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

        // Generate error assertions
        let error_assertions = test.errors.iter().map(|(line, col, msg)| {
            quote! {
                assert_error(&processor, #line, #col, #msg);
            }
        });

        quote! {
            #[test]
            fn #test_name() -> Result<(), String> {
                let input: Vec<u8> = vec![#(#input),*];
                let expected: Vec<u8> = vec![#(#expected),*];

                let mut processor = HtmlProcessor::create_full_parser(&input, "UTF-8").expect("Failed to create HTML processor");
                let actual = build_tree_representation(&mut processor)?;

                pretty_assertions::assert_str_eq!(
                    String::from_utf8_lossy(&expected),
                    String::from_utf8_lossy(&actual),
                    "Error on input:\n{}",
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
        pub mod html5lib_tests {
            #(#all_tests)*
        }
    };

    expanded.into()
}
