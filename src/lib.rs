#![no_std]
#![allow(clippy::tabs_in_doc_comments)]
#![doc = include_str!("../README.md")]

mod iterator;
mod option;
mod result;
pub use iterator::Iter;
pub use option::StaticOption;
pub use result::StaticResult;
