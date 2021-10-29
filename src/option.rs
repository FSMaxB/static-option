use crate::StaticResult;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::mem::{swap, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::option::IntoIter;
use core::pin::Pin;
use core::ptr::drop_in_place;

/// Example on how `StaticOption` can be used, implementing compile time checked builder pattern:
/// (NOTE: If you actually want to use builders like that, I recommend the excellent `typed-builder` crate instead)
/// ```
/// use static_option::StaticOption;
///
/// #[derive(Debug, PartialEq)]
/// struct Point {
/// 	x: f64,
/// 	y: f64,
/// }
///
/// impl Point {
/// 	pub fn build() -> Builder<false, false> {
/// 		Builder {
/// 			x: Default::default(),
/// 			y: Default::default(),
/// 		}
/// 	}
/// }
///
/// #[must_use = "Finish building with `.build()` or call `.drop()` if you don't use the `Builder` anymore."]
/// struct Builder<const X: bool, const Y: bool> {
/// 	x: StaticOption<f64, X>,
/// 	y: StaticOption<f64, Y>,
/// }
///
/// impl<const Y: bool> Builder<false, Y> {
/// 	pub fn x(self, x: f64) -> Builder<true, Y> {
///  		Builder {
/// 			x: x.into(),
/// 			y: self.y,
/// 		}
/// 	}
/// }
///
/// impl<const X: bool> Builder<X, false> {
/// 	pub fn y(self, y: f64) -> Builder<X, true> {
///  		Builder {
/// 			x: self.x,
/// 			y: y.into(),
/// 		}
/// 	}
/// }
///
/// impl<const X: bool, const Y: bool> Builder<X, Y> {
/// 	pub fn drop(self) {
/// 		let Self {
/// 			x,
/// 			y,
/// 		} = self;
/// 		x.drop();
///  		y.drop();
/// 	}
/// }
///
/// impl Builder<true, true> {
/// 	pub fn build(self) -> Point {
/// 		Point {
/// 			x: self.x.into_inner(),
/// 			y: self.y.into_inner(),
/// 		}
/// 	}
/// }
///
/// let point = Point::build().x(1.0).y(2.0).build();
/// let expected = Point { x: 1.0, y: 2.0 };
/// assert_eq!(expected, point);
/// ```
#[must_use = "Call `.drop()` if you don't use the StaticOption, otherwise it's contents never get dropped."]
pub struct StaticOption<T, const IS_SOME: bool> {
	value: MaybeUninit<T>,
}

impl<T> StaticOption<T, true> {
	pub const fn some(value: T) -> Self {
		Self {
			value: MaybeUninit::new(value),
		}
	}

	pub fn into_inner(self) -> T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { self.value.assume_init() }
	}

	pub fn inner_ref(&self) -> &T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { &*self.value.as_ptr() }
	}

	pub fn inner_mut(&mut self) -> &mut T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { &mut *self.value.as_mut_ptr() }
	}

	pub fn as_ref(&self) -> StaticOption<&T, true> {
		StaticOption::some(self.inner_ref())
	}

	pub fn as_mut(&mut self) -> StaticOption<&mut T, true> {
		StaticOption::some(self.inner_mut())
	}

	pub fn as_pin_ref(self: Pin<&Self>) -> StaticOption<Pin<&T>, true> {
		// SAFETY: self.get_ref() is guaranteed to be pinned because it comes from `self`
		// which is pinned
		unsafe { StaticOption::some(Pin::new_unchecked(self.get_ref().inner_ref())) }
	}

	pub fn as_pin_mut(self: Pin<&mut Self>) -> StaticOption<Pin<&mut T>, true> {
		// SAFETY: self.get_mut() is guaranteed to be pinned because it comes from `self`
		// which is pinned
		unsafe { StaticOption::some(Pin::new_unchecked(self.get_unchecked_mut().inner_mut())) }
	}

	pub fn map<U, F>(self, function: F) -> StaticOption<U, true>
	where
		F: FnOnce(T) -> U,
	{
		StaticOption::some(function(self.into_inner()))
	}

	pub fn ok_or<E>(self, _error: E) -> StaticResult<T, E, true> {
		StaticResult::ok(self.into_inner())
	}

	pub fn ok_or_else<E, F>(self, _error: F) -> StaticResult<T, E, true>
	where
		F: FnOnce() -> E,
	{
		StaticResult::ok(self.into_inner())
	}

	pub const fn and<U, const IS_SOME: bool>(self, option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, IS_SOME> {
		option_b
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, function: F) -> StaticOption<U, IS_SOME>
	where
		F: FnOnce(T) -> StaticOption<U, IS_SOME>,
	{
		function(self.into_inner())
	}

	pub fn or<const IS_SOME: bool>(self, _option_b: StaticOption<T, IS_SOME>) -> Self {
		self
	}

	pub fn or_else<F, const IS_SOME: bool>(self, _function: F) -> Self
	where
		F: FnOnce() -> StaticOption<T, IS_SOME>,
	{
		self
	}

	pub fn insert(&mut self, mut value: T) -> &mut T {
		swap(&mut value, self.inner_mut());
		self.inner_mut()
	}

	pub fn replace(&mut self, mut value: T) -> StaticOption<T, true> {
		swap(self.inner_mut(), &mut value);
		StaticOption::some(value)
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		self.into_inner()
	}

	pub fn as_deref(&self) -> StaticOption<&<T as Deref>::Target, true>
	where
		T: Deref,
	{
		StaticOption::some(self.inner_ref().deref())
	}

	pub fn as_deref_mut(&mut self) -> StaticOption<&mut <T as Deref>::Target, true>
	where
		T: DerefMut,
	{
		StaticOption::some(self.inner_mut().deref_mut())
	}
}

impl<'a, T> StaticOption<&'a T, true> {
	pub fn copied(self) -> StaticOption<T, true>
	where
		T: Copy,
	{
		StaticOption::some(**self.inner_ref())
	}

	pub fn cloned(self) -> StaticOption<T, true>
	where
		T: Clone,
	{
		StaticOption::some((*self.inner_ref()).clone())
	}
}

impl<T> StaticOption<T, false> {
	pub const fn none() -> Self {
		Self {
			value: MaybeUninit::uninit(),
		}
	}

	pub const fn as_ref(&self) -> StaticOption<&T, false> {
		StaticOption::none()
	}

	pub fn as_mut(&mut self) -> StaticOption<&mut T, false> {
		StaticOption::none()
	}

	pub fn as_pin_ref(self: Pin<&Self>) -> StaticOption<Pin<&T>, false> {
		StaticOption::none()
	}

	pub fn as_pin_mut(self: Pin<&mut Self>) -> StaticOption<Pin<&mut T>, false> {
		StaticOption::none()
	}

	pub fn map<U, F>(self, _function: F) -> StaticOption<U, false>
	where
		F: FnOnce(T) -> U,
	{
		StaticOption::none()
	}

	pub fn ok_or<E>(self, error: E) -> StaticResult<T, E, false> {
		StaticResult::err(error)
	}

	pub fn ok_or_else<E, F>(self, error: F) -> StaticResult<T, E, false>
	where
		F: FnOnce() -> E,
	{
		StaticResult::err(error())
	}

	pub const fn and<U, const IS_SOME: bool>(self, _option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, false> {
		StaticOption::none()
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, _function: F) -> StaticOption<U, false>
	where
		F: FnOnce(T) -> StaticOption<U, IS_SOME>,
	{
		StaticOption::none()
	}

	pub fn or<const IS_SOME: bool>(self, option_b: StaticOption<T, IS_SOME>) -> StaticOption<T, IS_SOME> {
		option_b
	}

	pub fn or_else<F, const IS_SOME: bool>(self, function: F) -> StaticOption<T, IS_SOME>
	where
		F: FnOnce() -> StaticOption<T, IS_SOME>,
	{
		function()
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		T::default()
	}

	pub fn as_deref(&self) -> StaticOption<&<T as Deref>::Target, false>
	where
		T: Deref,
	{
		StaticOption::none()
	}

	pub fn as_deref_mut(&mut self) -> StaticOption<&mut <T as Deref>::Target, false>
	where
		T: DerefMut,
	{
		StaticOption::none()
	}
}

impl<'a, T> StaticOption<&'a T, false> {
	pub fn copied(self) -> StaticOption<T, false>
	where
		T: Copy,
	{
		StaticOption::none()
	}

	pub fn cloned(self) -> StaticOption<T, false>
	where
		T: Clone,
	{
		StaticOption::none()
	}
}

impl<T, const IS_SOME: bool> StaticOption<T, IS_SOME> {
	pub const fn is_some(&self) -> bool {
		IS_SOME
	}

	pub const fn is_none(&self) -> bool {
		!IS_SOME
	}

	pub fn expect(self, message: &str) -> T {
		self.into_option().expect(message)
	}

	pub fn unwrap(self) -> T {
		if IS_SOME {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			unsafe { self.value.assume_init() }
		} else {
			panic!("called `StaticOption::unwrap()` on a `None` value")
		}
	}

	pub fn unwrap_or(self, default: T) -> T {
		self.into_option().unwrap_or(default)
	}

	pub fn unwrap_or_else<F>(self, function: F) -> T
	where
		F: FnOnce() -> T,
	{
		self.into_option().unwrap_or_else(function)
	}

	pub fn map_or<U, F>(self, default: U, function: F) -> U
	where
		F: FnOnce(T) -> U,
	{
		self.into_option().map_or(default, function)
	}

	pub fn map_or_else<U, D, F>(self, default: D, function: F) -> U
	where
		F: FnOnce(T) -> U,
		D: FnOnce() -> U,
	{
		self.into_option().map_or_else(default, function)
	}

	pub fn drop(mut self) {
		if IS_SOME {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			unsafe { drop_in_place(self.value.as_mut_ptr()) }
		}
	}

	pub fn into_option(self) -> Option<T> {
		if IS_SOME {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { self.value.assume_init() })
		} else {
			None
		}
	}

	pub fn as_option(&self) -> Option<&T> {
		if IS_SOME {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { &*self.value.as_ptr() })
		} else {
			None
		}
	}

	pub fn as_mut_option(&mut self) -> Option<&mut T> {
		if IS_SOME {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { &mut *self.value.as_mut_ptr() })
		} else {
			None
		}
	}
}

impl<T> Default for StaticOption<T, false> {
	fn default() -> Self {
		Self {
			value: MaybeUninit::uninit(),
		}
	}
}

impl<T, const IS_SOME: bool> From<StaticOption<T, IS_SOME>> for Option<T> {
	fn from(static_option: StaticOption<T, IS_SOME>) -> Self {
		static_option.into_option()
	}
}

impl<T> Clone for StaticOption<T, true>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		StaticOption::some(self.inner_ref().clone())
	}
}

impl<T> Clone for StaticOption<T, false>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		StaticOption::none()
	}
}

impl<T, const IS_SOME: bool> Debug for StaticOption<T, IS_SOME>
where
	T: Debug,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
		match self.as_option() {
			Some(value) => formatter.debug_tuple("StaticOption::some").field(value).finish(),
			None => formatter.debug_tuple("StaticOption::none").finish(),
		}
	}
}

impl<'a, T> From<&'a StaticOption<T, true>> for StaticOption<&'a T, true> {
	fn from(static_option: &'a StaticOption<T, true>) -> Self {
		static_option.as_ref()
	}
}

impl<'a, T> From<&'a StaticOption<T, false>> for StaticOption<&'a T, false> {
	fn from(static_option: &'a StaticOption<T, false>) -> Self {
		static_option.as_ref()
	}
}

impl<'a, T> From<&'a mut StaticOption<T, true>> for StaticOption<&'a mut T, true> {
	fn from(static_option: &'a mut StaticOption<T, true>) -> Self {
		static_option.as_mut()
	}
}

impl<'a, T> From<&'a mut StaticOption<T, false>> for StaticOption<&'a mut T, false> {
	fn from(static_option: &'a mut StaticOption<T, false>) -> Self {
		static_option.as_mut()
	}
}

impl<'a, T, const IS_SOME: bool> From<&'a StaticOption<T, IS_SOME>> for Option<&'a T> {
	fn from(static_option: &'a StaticOption<T, IS_SOME>) -> Self {
		static_option.as_option()
	}
}

impl<'a, T, const IS_SOME: bool> From<&'a mut StaticOption<T, IS_SOME>> for Option<&'a mut T> {
	fn from(static_option: &'a mut StaticOption<T, IS_SOME>) -> Self {
		static_option.as_mut_option()
	}
}

impl<T> From<T> for StaticOption<T, true> {
	fn from(value: T) -> Self {
		StaticOption::some(value)
	}
}

impl<T, const IS_SOME: bool> Hash for StaticOption<T, IS_SOME>
where
	T: Hash,
{
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_option().hash(state)
	}
}

impl<T, const IS_SOME: bool> IntoIterator for StaticOption<T, IS_SOME> {
	type Item = T;
	type IntoIter = IntoIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.into_option().into_iter()
	}
}

impl<T, const IS_SOME: bool> PartialEq for StaticOption<T, IS_SOME>
where
	T: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.as_option().eq(&other.as_option())
	}
}

impl<T, const IS_SOME: bool> PartialOrd for StaticOption<T, IS_SOME>
where
	T: PartialOrd,
{
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.as_option().partial_cmp(&other.as_option())
	}
}

impl<T, const IS_SOME: bool> Eq for StaticOption<T, IS_SOME> where T: Eq {}

impl<T, const IS_SOME: bool> Ord for StaticOption<T, IS_SOME>
where
	T: Ord,
{
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_option().cmp(&other.as_option())
	}
}

impl<T> Copy for StaticOption<T, true> where T: Copy {}
impl<T> Copy for StaticOption<T, false> where T: Copy {}
