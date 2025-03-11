use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::{collections::BTreeMap, fs};
use syn::{parse_macro_input, LitStr};

#[derive(Deserialize)]
struct DeserializedJSONEntity {
    characters: Box<str>,
}

fn process_file(file_path: &str) -> BTreeMap<[u8; 2], Vec<(Vec<u8>, Vec<u8>)>> {
    // Read the JSON file
    let json_content = fs::read_to_string(file_path)
        .unwrap_or_else(|_| panic!("Failed to read file: {}", file_path));

    // Parse the JSON file
    let entities: BTreeMap<String, DeserializedJSONEntity> =
        serde_json::from_str(&json_content).expect("Failed to parse JSON");

    // Group by prefix
    let mut prefix_map: BTreeMap<[u8; 2], Vec<(Vec<u8>, Vec<u8>)>> = BTreeMap::new();

    for (entity_name, entity_data) in entities {
        // Skip the '&' and take the first 2 characters as the prefix
        if entity_name.len() <= 1 {
            continue; // Skip if entity name is too short (just '&')
        }

        let entity_without_amp = &entity_name[1..]; // Skip the '&'

        // We can assume all entities have at least 2 bytes after the '&'
        let entity_bytes = entity_without_amp.as_bytes();
        let prefix = [entity_bytes[0], entity_bytes[1]]; // First 2 bytes as array

        // The rest of the bytes are the suffix
        let suffix = if entity_bytes.len() > 2 {
            entity_bytes[2..].to_vec()
        } else {
            Vec::new()
        };

        // Convert characters to UTF-8 bytes
        let bytes = entity_data.characters.as_bytes().to_vec();

        // Add to prefix map
        prefix_map.entry(prefix).or_default().push((suffix, bytes));
    }

    // Sort each group by suffix length (longer first)
    for entries in prefix_map.values_mut() {
        entries.sort_by(|(a, _), (b, _)| b.len().cmp(&a.len()));
    }

    prefix_map
}

#[proc_macro]
pub fn entities_lookup(input: TokenStream) -> TokenStream {
    // Parse the input to get the file path
    let file_path = parse_macro_input!(input as LitStr).value();

    // Process the file
    let prefix_map = process_file(&file_path);

    // Generate the BTreeMap initialization code
    let mut prefix_entries = Vec::new();

    // For each prefix, generate the entry code
    for (prefix, suffixes) in prefix_map {
        let prefix_bytes = [prefix[0], prefix[1]];
        let mut suffix_entries = Vec::new();

        // For each suffix in this prefix group
        for (suffix, bytes) in suffixes {
            let suffix_bytes: Vec<_> = suffix.iter().map(|&b| quote! { #b }).collect();
            let char_bytes: Vec<_> = bytes.iter().map(|&b| quote! { #b }).collect();

            // Create an entry with Box::leak to ensure 'static lifetime
            suffix_entries.push(quote! {
                (
                    Box::leak(Box::new([#(#suffix_bytes),*])) as &'static [u8],
                    Box::leak(Box::new([#(#char_bytes),*])) as &'static [u8]
                )
            });
        }

        // Create the prefix entry with Box::leak for the slice of pairs
        prefix_entries.push(quote! {
            ([#(#prefix_bytes),*], Box::leak(Box::new([#(#suffix_entries),*])) as &'static [(&'static [u8], &'static [u8])])
        });
    }

    // Generate the final TokenStream
    let result = quote! {
        use lazy_static::lazy_static;
        use std::collections::BTreeMap;

        lazy_static! {
            static ref ENTITIES: BTreeMap<[u8; 2], &'static [(&'static [u8], &'static [u8])]> = {
                let mut map = BTreeMap::new();
                #(map.insert(#prefix_entries.0, #prefix_entries.1);)*
                map
            };
        }
    };

    result.into()
}
