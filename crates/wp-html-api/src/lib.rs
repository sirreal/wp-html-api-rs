mod html_api;
use ext_php_rs::{builders::ModuleBuilder, prelude::*, types::ZendClassObject};

extern "C" fn request_startup(_ty: i32, _module_number: i32) -> i32 {
    0
}

extern "C" fn request_shutdown(_ty: i32, _module_number: i32) -> i32 {
    0
}

#[php_class]
pub struct WP_HTML_Processor_RS {
    processor: html_api::HtmlProcessor,
}

#[php_impl(rename_methods = "none")]
impl WP_HTML_Processor_RS {
    pub fn create_fragment(html: &str) -> Self {
        let processor = html_api::HtmlProcessor::create_fragment(html);
        Self { processor }
    }

    pub fn next_token(#[this] this: &mut ZendClassObject<WP_HTML_Processor_RS>) -> bool {
        this.processor.next_token()
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    let module = module
        .request_startup_function(request_startup)
        .request_shutdown_function(request_shutdown);
    module
}
