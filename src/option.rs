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
/// 			x: self.x.get(),
/// 			y: self.y.get(),
/// 		}
/// 	}
/// }
///
/// let point = Point::build().x(1.0).y(2.0).build();
/// let expected = Point { x: 1.0, y: 2.0 };
/// assert_eq!(expected, point);
/// ```
#[must_use = "Call `.drop()` if you don't use the StaticOption, otherwise it's contents never get dropped."]
pub struct StaticOption<T, const STATE: bool> {
	value: MaybeUninit<T>,
}

impl<T> StaticOption<T, true> {
	pub const fn some(value: T) -> Self {
		Self {
			value: MaybeUninit::new(value),
		}
	}

	pub fn get(self) -> T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { self.value.assume_init() }
	}

	pub fn get_ref(&self) -> &T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { self.value.assume_init_ref() }
	}

	pub fn get_mut(&mut self) -> &mut T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { self.value.assume_init_mut() }
	}

	pub fn as_ref(&self) -> StaticOption<&T, true> {
		StaticOption::some(self.get_ref())
	}

	pub fn as_mut(&mut self) -> StaticOption<&mut T, true> {
		StaticOption::some(self.get_mut())
	}

	pub fn as_pin_ref(self: Pin<&Self>) -> StaticOption<Pin<&T>, true> {
		// SAFETY: self.get_ref() is guaranteed to be pinned because it comes from `self`
		// which is pinned
		unsafe { StaticOption::some(Pin::new_unchecked(self.get_ref().get_ref())) }
	}

	pub fn as_pin_mut(self: Pin<&mut Self>) -> StaticOption<Pin<&mut T>, true> {
		// SAFETY: self.get_mut() is guaranteed to be pinned because it comes from `self`
		// which is pinned
		unsafe { StaticOption::some(Pin::new_unchecked(self.get_unchecked_mut().get_mut())) }
	}

	pub fn ok_or<E>(self, _error: E) -> StaticResult<T, E, true> {
		StaticResult::ok(self.get())
	}

	pub fn ok_or_else<E, F>(self, _error: F) -> StaticResult<T, E, true>
	where
		F: FnOnce() -> E,
	{
		StaticResult::ok(self.get())
	}

	pub const fn and<U, const STATE: bool>(self, option_b: StaticOption<U, STATE>) -> StaticOption<U, STATE> {
		option_b
	}

	pub fn and_then<U, F, const STATE: bool>(self, function: F) -> StaticOption<U, STATE>
	where
		F: FnOnce(T) -> StaticOption<U, STATE>,
	{
		function(self.get())
	}

	pub fn or<const STATE: bool>(self, _option_b: StaticOption<T, STATE>) -> Self {
		self
	}

	pub fn or_else<F, const STATE: bool>(self, _function: F) -> Self
	where
		F: FnOnce() -> StaticOption<T, STATE>,
	{
		self
	}

	pub fn insert(&mut self, mut value: T) -> &mut T {
		swap(&mut value, self.get_mut());
		self.get_mut()
	}

	pub fn replace(&mut self, mut value: T) -> StaticOption<T, true> {
		swap(self.get_mut(), &mut value);
		StaticOption::some(value)
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		self.get()
	}

	pub fn as_deref(&self) -> StaticOption<&<T as Deref>::Target, true>
	where
		T: Deref,
	{
		StaticOption::some(self.get_ref().deref())
	}

	pub fn as_deref_mut(&mut self) -> StaticOption<&mut <T as Deref>::Target, true>
	where
		T: DerefMut,
	{
		StaticOption::some(self.get_mut().deref_mut())
	}
}

impl<'a, T> StaticOption<&'a T, true> {
	pub fn copied(self) -> StaticOption<T, true>
	where
		T: Copy,
	{
		StaticOption::some(**self.get_ref())
	}

	pub fn cloned(self) -> StaticOption<T, true>
	where
		T: Clone,
	{
		StaticOption::some((*self.get_ref()).clone())
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

	pub fn ok_or<E>(self, error: E) -> StaticResult<T, E, false> {
		StaticResult::err(error)
	}

	pub fn ok_or_else<E, F>(self, error: F) -> StaticResult<T, E, false>
	where
		F: FnOnce() -> E,
	{
		StaticResult::err(error())
	}

	pub const fn and<U, const STATE: bool>(self, _option_b: StaticOption<U, STATE>) -> StaticOption<U, false> {
		StaticOption::none()
	}

	pub fn and_then<U, F, const STATE: bool>(self, _function: F) -> StaticOption<U, false>
	where
		F: FnOnce(T) -> StaticOption<U, STATE>,
	{
		StaticOption::none()
	}

	pub fn or<const STATE: bool>(self, option_b: StaticOption<T, STATE>) -> StaticOption<T, STATE> {
		option_b
	}

	pub fn or_else<F, const STATE: bool>(self, function: F) -> StaticOption<T, STATE>
	where
		F: FnOnce() -> StaticOption<T, STATE>,
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

impl<T, const STATE: bool> StaticOption<T, STATE> {
	pub const fn is_some(&self) -> bool {
		STATE
	}

	pub const fn is_none(&self) -> bool {
		!STATE
	}

	pub fn expect(self, message: &str) -> T {
		self.into_option().expect(message)
	}

	pub fn unwrap(self) -> T {
		if STATE {
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
		if STATE {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			unsafe { drop_in_place(self.value.as_mut_ptr()) }
		}
	}

	pub fn into_option(self) -> Option<T> {
		if STATE {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { self.value.assume_init() })
		} else {
			None
		}
	}

	pub fn as_option(&self) -> Option<&T> {
		if STATE {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { self.value.assume_init_ref() })
		} else {
			None
		}
	}

	pub fn as_mut_option(&mut self) -> Option<&mut T> {
		if STATE {
			// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
			Some(unsafe { self.value.assume_init_mut() })
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

impl<T, const STATE: bool> From<StaticOption<T, STATE>> for Option<T> {
	fn from(static_option: StaticOption<T, STATE>) -> Self {
		static_option.into_option()
	}
}

impl<T> Clone for StaticOption<T, true>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		StaticOption::some(self.get_ref().clone())
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

impl<T, const STATE: bool> Debug for StaticOption<T, STATE>
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

impl<'a, T, const STATE: bool> From<&'a StaticOption<T, STATE>> for Option<&'a T> {
	fn from(static_option: &'a StaticOption<T, STATE>) -> Self {
		static_option.as_option()
	}
}

impl<'a, T, const STATE: bool> From<&'a mut StaticOption<T, STATE>> for Option<&'a mut T> {
	fn from(static_option: &'a mut StaticOption<T, STATE>) -> Self {
		static_option.as_mut_option()
	}
}

impl<T> From<T> for StaticOption<T, true> {
	fn from(value: T) -> Self {
		StaticOption::some(value)
	}
}

impl<T, const STATE: bool> Hash for StaticOption<T, STATE>
where
	T: Hash,
{
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_option().hash(state)
	}
}

impl<T, const STATE: bool> IntoIterator for StaticOption<T, STATE> {
	type Item = T;
	type IntoIter = IntoIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.into_option().into_iter()
	}
}

impl<T, const STATE: bool> PartialEq for StaticOption<T, STATE>
where
	T: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.as_option().eq(&other.as_option())
	}
}

impl<T, const STATE: bool> PartialOrd for StaticOption<T, STATE>
where
	T: PartialOrd,
{
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.as_option().partial_cmp(&other.as_option())
	}
}

impl<T, const STATE: bool> Eq for StaticOption<T, STATE> where T: Eq {}

impl<T, const STATE: bool> Ord for StaticOption<T, STATE>
where
	T: Ord,
{
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_option().cmp(&other.as_option())
	}
}

impl<T> Copy for StaticOption<T, true> where T: Copy {}
impl<T> Copy for StaticOption<T, false> where T: Copy {}
