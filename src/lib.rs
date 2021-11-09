#![no_std]
#![allow(clippy::tabs_in_doc_comments)]
#![doc = include_str!("../README.md")]

mod iterator;
mod option;
mod result;
pub use iterator::Iter;
pub use option::StaticOption;
pub use result::StaticResult;

/// Ensures that no code following the `const_assert` gets executed. Since panicking
/// in const fn's isn't stable yet in 1.56.
#[inline(always)]
pub(crate) const fn const_assert(condition: bool) {
	// Hack to make the compilation fail if condition is false.
	// This works by casting !true to 0usize, which compiles and !false to 1usize which accesses the array out of range.
	[()][(!condition) as usize]
}
