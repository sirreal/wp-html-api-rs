use super::{
    ActiveFormattingElements, EncodingConfidence, InsertionMode, StackOfOpenElements,
    html_token::HTMLToken,
};

pub(super) struct ProcessorState {
    pub(super) active_formatting_elements: ActiveFormattingElements,
    pub(super) current_token: Option<HTMLToken>,
    pub(super) encoding: Box<str>,
    pub(super) encoding_confidence: EncodingConfidence,
    pub(super) form_element: Option<HTMLToken>,
    pub(super) frameset_ok: bool,
    pub(super) head_element: Option<HTMLToken>,
    pub(super) insertion_mode: InsertionMode,
    pub(super) stack_of_open_elements: StackOfOpenElements,
    pub(super) stack_of_template_insertion_modes: Vec<InsertionMode>,
}
impl ProcessorState {
    pub(super) fn new() -> Self {
        Self {
            active_formatting_elements: ActiveFormattingElements::new(),
            current_token: None,
            encoding: "UTF-8".into(),
            encoding_confidence: EncodingConfidence::Tentative,
            form_element: None,
            frameset_ok: true,
            head_element: None,
            insertion_mode: InsertionMode::INITIAL,
            stack_of_open_elements: StackOfOpenElements::new(),
            stack_of_template_insertion_modes: Vec::new(),
        }
    }
}
