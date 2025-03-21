use wp_html_api::{
    html_processor::errors::HtmlProcessorError,
    tag_name::TagName,
    tag_processor::{AttributeValue, ParsingNamespace, TokenType},
};

const TREE_INDENT: &[u8] = b"  ";

pub fn build_tree_representation(
    processor: &mut wp_html_api::html_processor::HtmlProcessor,
) -> Result<Vec<u8>, TreeBuilderError> {
    let mut output: Vec<u8> = Vec::new();
    let mut indent_level = 0;
    let mut was_text = false;
    let mut text_node: Vec<u8> = Vec::new();

    enum Reached {
        None,
        Html,
        Head,
        BodyOrFrameset,
    }

    let mut reached = Reached::None;
    while processor.next_token() {
        if processor.get_last_error().is_some() {
            break;
        }

        let token_type = processor.get_token_type();

        if was_text && Some(&TokenType::Text) != token_type {
            if !text_node.is_empty() {
                output.extend(text_node.drain(..));
                output.extend(b"\"\n");
            }
            was_text = false;
        }

        match token_type {
            Some(TokenType::Doctype) => {
                let doctype = processor
                    .get_doctype_info()
                    .ok_or("Failed to process DOCTYPE token")?;
                output.extend(b"<!DOCTYPE ");
                if let Some(name) = doctype.name {
                    output.extend(name);
                }

                if doctype.public_identifier.is_some() || doctype.system_identifier.is_some() {
                    output.extend(b" \"");
                    output.extend(doctype.public_identifier.unwrap_or_default());
                    output.extend(b"\" \"");
                    output.extend(doctype.system_identifier.unwrap_or_default());
                    output.extend(b"\"");
                }
                output.extend(b">\n");
            }

            Some(TokenType::Tag) => {
                let namespace = processor.get_namespace();
                let tag_name = processor.get_tag().ok_or("Failed to get tag name")?;
                let printable_tag_name = if namespace == &ParsingNamespace::Html {
                    let s: Box<[u8]> = (&tag_name).into();
                    s.to_ascii_lowercase()
                } else {
                    let s: String = namespace.into();
                    let mut s: Vec<u8> = s.into();
                    s.push(b' ');
                    let qualified_tag_name = processor
                        .get_qualified_tag_name()
                        .ok_or("Failed to get qualified tag name ")?;
                    s.extend(qualified_tag_name);
                    s
                };

                if processor.is_tag_closer() {
                    indent_level -= 1;
                    if namespace == &ParsingNamespace::Html && tag_name == TagName::TEMPLATE {
                        indent_level -= 1;
                    }
                    continue;
                }

                match tag_name {
                    TagName::HTML => reached = Reached::Html,
                    TagName::HEAD => reached = Reached::Head,
                    TagName::BODY | TagName::FRAMESET => reached = Reached::BodyOrFrameset,
                    _ => {}
                }

                let tag_indent = indent_level;
                if processor
                    .expects_closer(None)
                    .ok_or("Failed to get expects closer")?
                {
                    indent_level += 1;
                }

                output.extend(TREE_INDENT.repeat(tag_indent));
                output.push(b'<');
                output.extend(printable_tag_name);
                output.extend(b">\n");
                // Handle attributes
                match processor.get_attribute_names_with_prefix(b"") {
                    Some(attribute_names) if !attribute_names.is_empty() => {
                        let mut attribute_names = attribute_names
                            .iter()
                            .map(|name| {
                                (
                                    name,
                                    processor
                                        .get_qualified_attribute_name(name)
                                        .expect("Failed to get qualified attribute name"),
                                )
                            })
                            .collect::<Vec<_>>();

                        /*
                         * Sorts attributes to match html5lib sort order.
                         *
                         *  - First comes normal HTML attributes.
                         *  - Then come adjusted foreign attributes; these have spaces in their names.
                         *  - Finally come non-adjusted foreign attributes; these have a colon in their names.
                         *
                         * Example:
                         *
                         *       From: <math xlink:author definitionurl xlink:title xlink:show>
                         *     Sorted: 'definitionURL', 'xlink show', 'xlink title', 'xlink:author'
                         */
                        attribute_names.sort_by(|(_, a_display), (_, b_display)| {
                            use std::cmp::Ordering as O;
                            let a_has_ns = a_display.contains(&b':');
                            let b_has_ns = b_display.contains(&b':');

                            // Attributes with `:` should follow all other attributes.
                            if a_has_ns != b_has_ns {
                                return if a_has_ns { O::Greater } else { O::Less };
                            }

                            // Attributes with a namespace ' ' should come after those without.
                            let a_has_sp = a_display.contains(&b' ');
                            let b_has_sp = b_display.contains(&b' ');
                            if a_has_sp != b_has_sp {
                                return if a_has_sp { O::Greater } else { O::Less };
                            }

                            a_display.cmp(b_display)
                        });

                        for (name, display_name) in attribute_names {
                            let val: &[u8] = match processor
                                .get_attribute(name)
                                .ok_or("Failed to get attribute value")?
                            {
                                AttributeValue::BooleanFalse => unreachable!(
                                    "Expected set attribute when procissing attribute names."
                                ),
                                /*
                                 * Attributes with no value use the empty string value
                                 * in the tree structure.
                                 */
                                AttributeValue::BooleanTrue => b"",
                                AttributeValue::String(value) => &value.clone(),
                            };
                            output.extend(TREE_INDENT.repeat(tag_indent + 1));
                            output.extend(display_name);
                            output.extend(b"=\"");
                            output.extend(val);
                            output.extend(b"\"\n");
                        }
                    }
                    _ => {}
                };

                let modifiable_text = processor.get_modifiable_text();
                if !modifiable_text.is_empty() {
                    output.extend(TREE_INDENT.repeat(tag_indent + 1));
                    output.push(b'"');
                    output.extend(modifiable_text);
                    output.extend(b"\"\n");
                }

                if namespace == &ParsingNamespace::Html && tag_name == TagName::TEMPLATE {
                    output.extend(TREE_INDENT.repeat(indent_level));
                    output.extend(b"content\n");
                    indent_level += 1;
                }
            }

            Some(TokenType::CdataSection | TokenType::Text) => {
                let text_content = processor.get_modifiable_text();
                if text_content.is_empty() {
                    continue;
                }
                was_text = true;
                if text_node.is_empty() {
                    text_node.extend(TREE_INDENT.repeat(indent_level));
                    text_node.push(b'"');
                }
                text_node.extend(text_content);
            }

            Some(TokenType::FunkyComment) => {
                // Comments must be "<" then "!-- " then the data then " -->".
                output.extend(TREE_INDENT.repeat(indent_level));
                output.extend(b"<!-- ");
                output.extend(processor.get_modifiable_text());
                output.extend(b" -->\n");
            }

            Some(TokenType::Comment) => {
                // Comments must be "<" then "!-- " then the data then " -->".
                output.extend(TREE_INDENT.repeat(indent_level));
                output.extend(b"<!-- ");
                output.extend(
                    processor
                        .get_full_comment_text()
                        .ok_or("Failed to get comment text")?,
                );
                output.extend(b" -->\n");
            }

            Some(TokenType::PresumptuousTag) => {
                // </> is ignored in HTML.
            }

            None => Err("Got None, expected a token type.")?,
        }
    }

    if let Some(error) = processor.get_last_error() {
        Err(error)?;
    }

    if processor.paused_at_incomplete_token() {
        Err(TreeBuilderError::PausedAtIncompleteToken)?;
    }

    if !text_node.is_empty() {
        output.extend(text_node.drain(..));
        output.extend(b"\"\n");
    }

    match reached {
        Reached::None => output.extend(b"<html>\n  <head>\n  <body>\n"),
        Reached::Html => output.extend(b"  <head>\n  <body>\n"),
        Reached::Head => output.extend(b"  <body>\n"),
        Reached::BodyOrFrameset => {}
    }

    // Tests always end with a trailing newline
    output.push(b'\n');
    Ok(output)
}

pub enum TreeBuilderError {
    PausedAtIncompleteToken,
    Arbitrary(String),
    HtmlProcessor(HtmlProcessorError),
}
impl From<&str> for TreeBuilderError {
    fn from(s: &str) -> Self {
        TreeBuilderError::Arbitrary(s.to_string())
    }
}
impl From<&HtmlProcessorError> for TreeBuilderError {
    fn from(err: &HtmlProcessorError) -> Self {
        TreeBuilderError::HtmlProcessor(err.clone())
    }
}
impl From<TreeBuilderError> for String {
    fn from(err: TreeBuilderError) -> String {
        match err {
            TreeBuilderError::PausedAtIncompleteToken => "Paused at incomplete token.".into(),
            TreeBuilderError::Arbitrary(s) => s,
            TreeBuilderError::HtmlProcessor(err) => match err {
                HtmlProcessorError::ExceededMaxBookmarks => {
                    let s: &str = err.into();
                    s.into()
                }
                HtmlProcessorError::UnsupportedException(unsupported_exception) => {
                    let s: &str = unsupported_exception.into();
                    s.into()
                }
            },
        }
    }
}
