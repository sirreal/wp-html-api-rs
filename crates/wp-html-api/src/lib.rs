#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[macro_use]
pub(crate) mod macros;
pub(crate) mod attributes;
pub(crate) mod str_fns;

pub mod compat_mode;
pub mod doctype;
pub mod html_processor;
pub mod tag_name;
pub mod tag_processor;
