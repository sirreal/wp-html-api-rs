use crate::tag_processor::TagProcessor;

pub struct HtmlProcessor {
    tag_processor: TagProcessor,
}

impl HtmlProcessor {
    pub fn create_fragment(html: &str) -> Self {
        let tag_processor = TagProcessor::new(html);
        Self { tag_processor }
    }
}
