use quote::quote;

use std::fmt;
use std::fs;
use syn::{parse_macro_input, LitStr};

#[derive(PartialEq, Clone)]
pub enum Node {
    Element {
        name: String,
        attributes: Vec<(String, String)>,
        children: Vec<Node>,
    },
    Text(String),
    Comment(String),
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
    },
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                write!(f, "<{}", name)?;
                for (key, value) in attributes {
                    write!(f, " {}=\"{}\"", key, value)?;
                }
                write!(f, ">")?;
                if !children.is_empty() {
                    write!(f, "\n")?;
                    for child in children {
                        for line in format!("{:?}", child).lines() {
                            write!(f, "  {}\n", line)?;
                        }
                    }
                }
                write!(f, "</{}>", name)
            }
            Node::Text(text) => write!(f, "{:?}", text),
            Node::Comment(text) => write!(f, "<!-- {} -->", text),
            Node::Doctype {
                name,
                public_id,
                system_id,
            } => {
                write!(f, "<!DOCTYPE")?;
                if let Some(name) = name {
                    write!(f, " {}", name)?;
                }
                if let Some(public_id) = public_id {
                    write!(f, " PUBLIC \"{}\"", public_id)?;
                }
                if let Some(system_id) = system_id {
                    write!(f, " \"{}\"", system_id)?;
                }
                write!(f, ">")
            }
        }
    }
}

pub struct TestCase {
    pub input: String,
    pub errors: Vec<(usize, usize, String)>, // (line, col, message)
    pub expected_document: String,
}

pub fn parse_test_file(content: &str) -> Vec<TestCase> {
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

pub struct TreeBuilder {
    pub nodes: Vec<Node>,
    pub stack: Vec<usize>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        let root = Node::Element {
            name: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        };
        let mut nodes = Vec::new();
        nodes.push(root);
        Self {
            nodes,
            stack: vec![0],
        }
    }

    pub fn add_node(&mut self, node: Node) {
        let current_idx = *self.stack.last().unwrap();
        if let Node::Element { children, .. } = &mut self.nodes[current_idx] {
            children.push(node);
        }
    }

    pub fn push_element(&mut self, name: String, attributes: Vec<(String, String)>) {
        let node = Node::Element {
            name,
            attributes,
            children: Vec::new(),
        };
        let current_idx = *self.stack.last().unwrap();
        if let Node::Element { children, .. } = &mut self.nodes[current_idx] {
            children.push(node);
            if let Some(Node::Element { .. }) = children.last() {
                self.stack.push(children.len() - 1);
            }
        }
    }

    pub fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    pub fn finish(mut self) -> Node {
        if let Node::Element { mut children, .. } = self.nodes.remove(0) {
            children.remove(0)
        } else {
            panic!("Invalid root node");
        }
    }
}

pub fn parse_expected_document(content: &str) -> Node {
    let mut builder = TreeBuilder::new();
    let mut current_indent = 0;

    for line in content.lines() {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }

        let indent = line.chars().take_while(|&c| c == '|' || c == ' ').count();
        let content = line[indent..].trim_start();

        // Pop back up the stack if we're at a lower indent level
        while indent < current_indent && builder.stack.len() > 1 {
            builder.pop();
            current_indent -= 2;
        }

        if content.starts_with("<!DOCTYPE") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            builder.add_node(Node::Doctype {
                name: parts.get(1).map(|s| s.to_string()),
                public_id: None,
                system_id: None,
            });
        } else if content.starts_with('"') && content.ends_with('"') {
            // Text node
            let text = content[1..content.len() - 1].to_string();
            builder.add_node(Node::Text(text));
        } else if content.starts_with("<!--") && content.ends_with("-->") {
            // Comment node
            let comment = content[4..content.len() - 3].trim().to_string();
            builder.add_node(Node::Comment(comment));
        } else if content.starts_with('<') {
            // Element node
            let element_content = content[1..].trim_end_matches('>');
            let (name, attrs_str) = match element_content.find(' ') {
                Some(idx) => (&element_content[..idx], &element_content[idx + 1..]),
                None => (element_content, ""),
            };

            let mut attributes = Vec::new();
            // Parse each attribute
            for attr in attrs_str.split_whitespace() {
                if let Some((name, value)) = attr.split_once('=') {
                    attributes.push((name.to_string(), value.trim_matches('"').to_string()));
                }
            }

            builder.push_element(name.to_string(), attributes);
            current_indent = indent;
        }
    }

    builder.finish()
}

pub fn build_tree(processor: &mut wp_html_api::html_processor::HtmlProcessor) -> Node {
    let mut builder = TreeBuilder::new();

    while processor.next_token() {
        match processor.get_token_type() {
            Some(wp_html_api::tag_processor::TokenType::Tag) => {
                if processor.is_tag_closer() {
                    builder.pop();
                } else {
                    let tag_name = processor.get_tag().unwrap().to_string();
                    let attributes = Vec::new();
                    // TODO: Implement attribute handling once the API is available
                    builder.push_element(tag_name, attributes);
                }
            }
            Some(wp_html_api::tag_processor::TokenType::Text) => {
                let text = processor.get_modifiable_text();
                builder.add_node(Node::Text(String::from_utf8_lossy(&text).into()));
            }
            Some(wp_html_api::tag_processor::TokenType::Comment) => {
                let comment = processor.get_full_comment_text().unwrap();
                builder.add_node(Node::Comment(String::from_utf8_lossy(&comment).into()));
            }
            Some(wp_html_api::tag_processor::TokenType::Doctype) => {
                if let Some(doctype) = processor.get_doctype_info() {
                    builder.add_node(Node::Doctype {
                        name: doctype
                            .name
                            .map(|s| String::from_utf8_lossy(&s).into_owned()),
                        public_id: doctype
                            .public_identifier
                            .map(|s| String::from_utf8_lossy(&s).into_owned()),
                        system_id: doctype
                            .system_identifier
                            .map(|s| String::from_utf8_lossy(&s).into_owned()),
                    });
                }
            }
            _ => {}
        }
    }

    builder.finish()
}
