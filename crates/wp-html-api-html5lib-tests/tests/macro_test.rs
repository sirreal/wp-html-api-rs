//! This module contains generated HTML5Lib test cases

use wp_html_api_html5lib_tests::html5lib_tests;
use wp_html_api_html5lib_tests_gen_tests::{build_tree, parse_expected_document};

// Generate test functions from the test data file
html5lib_tests!("crates/wp-html-api-html5lib-tests/data/tree-construction/tests1.dat");
