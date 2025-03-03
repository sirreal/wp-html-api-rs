use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::{collections::BTreeMap, fs};
use syn::{parse_macro_input, LitStr};

#[derive(Deserialize)]
struct DesrializedJSONEntities {
    codepoints: Box<[u64]>,
    characters: Box<str>,
}

fn process_file(test_file_path: &str) -> Vec<(Box<[u8]>, Box<[u8]>)> {
    let content = fs::read(test_file_path).expect("Failed to read test file");
    let ents: BTreeMap<&str, DesrializedJSONEntities> =
        serde_json::from_slice(&content).expect("Failed to parse test file");
    let mut bytes_to_bytes: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for (html_ent, DesrializedJSONEntities { characters, .. }) in ents.iter() {
        let key = html_ent.bytes().collect::<Vec<_>>().into();
        let val = characters.bytes().collect::<Vec<_>>().into();
        bytes_to_bytes.push((key, val));
    }
    let bytes_to_bytes = bytes_to_bytes
        .into_iter()
        .map(|(k, v)| -> (Box<[u8]>, Box<[u8]>) { (k.into(), v.into()) })
        .collect();

    bytes_to_bytes
}

#[proc_macro]
pub fn entities_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let entity_map = process_file(&input.value());

    let (mut ts, mut vs) = (Vec::new(), Vec::new());
    entity_map.iter().for_each(|(k, v)| {
        let k_len = k.len();
        let v_len = v.len();
        ts.push(quote! { (&[u8], &[u8]) });
        vs.push(quote! { (&[#(#k),*], &[#(#v),*]) });
    });
    let len = entity_map.len();

    let val = quote! {
        lazy_static::lazy_static! {
            static ref entity_data: [(&'static [u8], &'static [u8]); #len] = [#(#vs),*];
            static ref mapped_entities: std::collections::BTreeMap<&'static [u8], &'static [u8]> = {
                let mut m = std::collections::BTreeMap::new();
                for (k, v) in entity_data.into_iter() {
                    m.insert(k, v);
                }
                m
            };
        }
    };
    val.into()
}
