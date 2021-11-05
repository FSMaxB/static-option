use crate::iterator::Iter;
use crate::StaticResult;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::mem::{swap, ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::drop_in_place;

#[must_use = "Call `.drop()` if you don't use the StaticOption, otherwise it's contents never get dropped."]
pub struct StaticOption<T, const IS_SOME: bool> {
	pub(crate) value: MaybeUninit<T>,
}

impl<T> StaticOption<T, true> {
	/// Create a [`StaticOption<T, true>`] with a value inside. The `true` type parameter statically tracks
	/// the fact that a value is inside.
	pub const fn some(value: T) -> Self {
		Self {
			value: MaybeUninit::new(value),
		}
	}

	/// Take out the value from a [`StaticOption<T, true>`]. This is possible because the `true` statically guarantees
	/// that there is a value inside.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// let inner: i32 = option.into_inner();
	/// assert_eq!(42, inner);
	/// ```
	pub fn into_inner(self) -> T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { self.value.assume_init() }
	}

	/// Take a shared borrow of the value inside a [`StaticOption<T, true>`]. This is possible because the `true` statically guarantees
	/// that there is a value inside that can be borrowed.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// let inner: &i32 = option.inner_ref();
	/// assert_eq!(42, *inner);
	/// ```
	pub fn inner_ref(&self) -> &T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { &*self.value.as_ptr() }
	}

	/// Take a mutable borrow of the value inside a [`StaticOption<T, true>`]. This is possible because the `true` statically guarantees
	/// that there is a value inside that can be borrowed.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let mut option = StaticOption::some(42);
	/// let inner: &mut i32 = option.inner_mut();
	/// *inner = 1337;
	/// assert_eq!(StaticOption::some(1337), option);
	/// ```
	pub fn inner_mut(&mut self) -> &mut T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		unsafe { &mut *self.value.as_mut_ptr() }
	}

	/// See [`core::option::Option::and`].
	///
	/// Return `option_b`.
	///
	/// Note that the `and` method on [`StaticOption<T, false>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option_a = StaticOption::some(42);
	/// let option_b = StaticOption::some("hello");
	///
	/// assert_eq!(StaticOption::some("hello"), option_a.and(option_b));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option_a = StaticOption::some(42);
	/// let option_b = StaticOption::<&'static str, false>::none();
	///
	/// assert_eq!(StaticOption::none(), option_a.and(option_b));
	/// ```
	pub const fn and<U, const IS_SOME: bool>(self, option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, IS_SOME> {
		option_b
	}

	/// See [`core::option::Option::and_then`].
	///
	/// Call the `mapper` function with the value contained in `self` and forward it's return value.
	///
	/// Note that the `and_then` method on [`StaticOption<T, false>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some("hello");
	/// let mapped = option.and_then(|text| StaticOption::some(text.len()));
	/// assert_eq!(StaticOption::some(5), mapped);
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// let mapped = option.and_then(|_| StaticOption::<&'static str, false>::none());
	/// assert_eq!(StaticOption::none(), mapped);
	/// ```
	pub fn and_then<U, F, const IS_SOME: bool>(self, mapper: F) -> StaticOption<U, IS_SOME>
	where
		F: FnOnce(T) -> StaticOption<U, IS_SOME>,
	{
		mapper(self.into_inner())
	}

	/// See [`core::option::Option::or`].
	///
	/// Return `self`, ignoring `_option_b`.
	///
	/// Note that the `or` method on [`StaticOption<T, false>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// let option_b = StaticOption::some(1337);
	/// assert_eq!(StaticOption::some(42), option.or(option_b));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// let option_b = StaticOption::none();
	/// assert_eq!(StaticOption::some(42), option.or(option_b));
	/// ```
	pub const fn or<const IS_SOME: bool>(self, _option_b: StaticOption<T, IS_SOME>) -> Self {
		self
	}

	/// See [`core::option::Option::or_else`].
	///
	/// Return `self`, ignoring `_fallback`.
	///
	/// Note that the `or_else` method on [`StaticOption<T, false>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// assert_eq!(StaticOption::some(42), option.or_else(|| StaticOption::some(1337)));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// assert_eq!(StaticOption::some(42), option.or_else(|| StaticOption::none()));
	/// ```
	pub fn or_else<F, const IS_SOME: bool>(self, _fallback: F) -> Self
	where
		F: FnOnce() -> StaticOption<T, IS_SOME>,
	{
		self
	}

	/// See [`core::option::Option::insert`]
	///
	/// Replace the current value in `self` and returns a mutable borrow to it.
	///
	/// Note that this method only exists on [`StaticOption<T, true>`] because a [`StaticOption<T, false>`] can
	/// never be modified to contain a value.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let mut option = StaticOption::some(42);
	/// let borrow: &mut i32 = option.insert(1337);
	/// assert_eq!(1337, *borrow);
	/// assert_eq!(StaticOption::some(1337), option);
	/// ```
	pub fn insert(&mut self, mut value: T) -> &mut T {
		swap(&mut value, self.inner_mut());
		self.inner_mut()
	}

	/// See [`core::option::Option::replace`].
	///
	/// Replace the current value in `self`, returning a [`StaticOption`] containing the previous value.
	///
	/// Note that this method only exists on [`StaticOption<T, true>'] because a [`StaticOption<T, false>`] can
	/// never be modified to contain a value.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let mut option = StaticOption::some(42);
	/// let original = option.replace(1337);
	/// assert_eq!(StaticOption::some(42), original);
	/// assert_eq!(StaticOption::some(1337), option);
	/// ```
	pub fn replace(&mut self, mut value: T) -> StaticOption<T, true> {
		swap(self.inner_mut(), &mut value);
		StaticOption::some(value)
	}
}

impl<T> StaticOption<T, false> {
	/// Create a [`StaticOption<T, false>`] without any value. The `false` type parameter statically tracks
	/// the fact that it contains no value.
	pub const fn none() -> Self {
		Self {
			value: MaybeUninit::uninit(),
		}
	}

	/// See [`core::option::Option::and`].
	///
	/// Return [`StaticOption<U, false>::none()`], ignoring `_option_b`.
	///
	/// Note that the `and` method on [`StaticOption<T, true>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&'static str, false>::none();
	/// let option_b = StaticOption::some(42);
	/// assert_eq!(StaticOption::<i32, false>::none(), option.and(option_b));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&'static str, false>::none();
	/// let option_b = StaticOption::<i32, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.and(option_b));
	/// ```
	pub const fn and<U, const IS_SOME: bool>(self, _option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, false> {
		StaticOption::none()
	}

	/// See [`core::option::Option::and_then`].
	///
	/// Return [`StaticOption<U, false>::none()`], ignoring `_mapper`.
	///
	/// Note that the `and_then` method on [`StaticOption<T, true>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&'static str, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.and_then(|_| StaticOption::some(42)));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&'static str, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.and_then(|_| StaticOption::<i32, false>::none()));
	/// ```
	pub fn and_then<U, F, const IS_SOME: bool>(self, _mapper: F) -> StaticOption<U, false>
	where
		F: FnOnce(T) -> StaticOption<U, IS_SOME>,
	{
		StaticOption::none()
	}

	/// See [`core::option::Option::or`].
	///
	/// Return `option_b`.
	///
	/// Note that the `or` method on [`StaticOption<T, true>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::none();
	/// let option_b = StaticOption::some(42);
	/// assert_eq!(StaticOption::some(42), option.or(option_b));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::none();
	/// let option_b = StaticOption::<i32, false>::none();
	/// assert_eq!(StaticOption::none(), option.or(option_b));
	/// ```
	pub const fn or<const IS_SOME: bool>(self, option_b: StaticOption<T, IS_SOME>) -> StaticOption<T, IS_SOME> {
		option_b
	}

	/// See [`core::option::Option::or_else`].
	///
	/// Call the `fallback` function and forward it's return value.
	///
	/// Note that the `or_else` method on [`StaticOption<T, true>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::none();
	/// assert_eq!(StaticOption::some(42), option.or_else(|| StaticOption::some(42)));
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.or_else(|| StaticOption::<i32, false>::none()));
	/// ```
	pub fn or_else<F, const IS_SOME: bool>(self, fallback: F) -> StaticOption<T, IS_SOME>
	where
		F: FnOnce() -> StaticOption<T, IS_SOME>,
	{
		fallback()
	}
}

impl<'a, T, const IS_SOME: bool> StaticOption<&'a T, IS_SOME> {
	/// See [`core::option::Option::copied`].
	///
	/// Take a [`StaticOption`] containing a reference and return a new [`StaticOption`]
	/// with an owned copy.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let number = 42;
	/// let option = StaticOption::some(&number);
	/// assert_eq!(StaticOption::some(42), option.copied());
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&i32, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.copied());
	/// ```
	pub fn copied(self) -> StaticOption<T, IS_SOME>
	where
		T: Copy,
	{
		StaticOption {
			value: match self.as_option() {
				Some(value) => MaybeUninit::new(**value),
				None => MaybeUninit::uninit(),
			},
		}
	}

	/// See [`core::option::Option::cloned`].
	///
	/// Take a [`StaticOption`] containing a reference and return a new [`StaticOption`]
	/// with an owned clone.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let text = String::from("hello");
	/// let option = StaticOption::some(&text);
	/// assert_eq!(StaticOption::some(String::from("hello")), option.cloned());
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<&String, false>::none();
	/// assert_eq!(StaticOption::<String, false>::none(), option.cloned());
	/// ```
	pub fn cloned(self) -> StaticOption<T, IS_SOME>
	where
		T: Clone,
	{
		StaticOption {
			value: match self.as_option() {
				Some(value) => MaybeUninit::new((*value).clone()),
				None => MaybeUninit::uninit(),
			},
		}
	}
}

impl<T, const IS_SOME: bool> StaticOption<StaticOption<T, IS_SOME>, true> {
	/// See [`core::option::Option::flatten`].
	///
	/// Return the contained [`StaticOption`].
	///
	/// Note that the `flatten` method on [`StaticOption<StaticOption<T, IS_SOME>, false>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(StaticOption::some(42));
	/// assert_eq!(StaticOption::some(42), option.flatten());
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(StaticOption::<i32, false>::none());
	/// assert_eq!(StaticOption::<i32, false>::none(), option.flatten());
	/// ```
	pub fn flatten(self) -> StaticOption<T, IS_SOME> {
		self.into_inner()
	}
}

impl<T, const IS_SOME: bool> StaticOption<StaticOption<T, IS_SOME>, false> {
	/// See [`core::option::Option::flatten`].
	///
	/// Return [`StaticOption::none()`].
	///
	/// Note that the `flatten` method on [`StaticOption<StaticOption<T, IS_SOME>, true>`] behaves differently.
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<StaticOption<i32, true>, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.flatten());
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<StaticOption::<i32, false>, false>::none();
	/// assert_eq!(StaticOption::<i32, false>::none(), option.flatten());
	/// ```
	pub const fn flatten(self) -> StaticOption<T, false> {
		StaticOption::none()
	}
}

impl<T, E, const IS_OK: bool> StaticOption<StaticResult<T, E, IS_OK>, true> {
	/// See [`core::option::Option::transpose`].
	///
	/// If the contained [`StaticResult`] is `ok`, return an `ok` result with a [`StaticOption::some`] of the `ok` value inside.
	/// Otherwise return a [`StaticResult`] with the original `error` value.
	///
	/// Note that the `transpose` method on [`StaticOption<StaticResult<T, E, IS_OK>, false>`] behaves differently.
	///
	///
	/// # Examples
	/// ```
	/// # use static_option::{StaticOption, StaticResult};
	/// let option = StaticOption::some(StaticResult::<_, &'static str, true>::new_ok(42));
	/// assert_eq!(StaticResult::new_ok(StaticOption::some(42)), option.transpose());
	/// ```
	///
	/// ```
	/// # use static_option::{StaticOption, StaticResult};
	/// let option = StaticOption::some(StaticResult::<i32, &'static str, false>::new_err("error"));
	/// assert_eq!(StaticResult::new_err("error"), option.transpose())
	/// ```
	pub fn transpose(self) -> StaticResult<StaticOption<T, true>, E, IS_OK> {
		let result = self.into_inner();
		result.map(StaticOption::some)
	}
}

impl<T, E, const IS_OK: bool> StaticOption<StaticResult<T, E, IS_OK>, false> {
	/// See [`core::option::Option::transpose`].
	///
	/// Return an `ok` [`StaticResult`] containing a [`StaticOption::none`].
	///
	/// Note that the `transpose` method on [`StaticOption<StaticResult<T, E, IS_OK>, true>`] behaves differently.
	///
	///
	/// # Examples
	/// ```
	/// # use static_option::{StaticOption, StaticResult};
	/// let option = StaticOption::<StaticResult<i32, &'static str, true>, false>::none();
	/// assert_eq!(StaticResult::new_ok(StaticOption::none()), option.transpose());
	/// ```
	///
	/// ```
	/// # use static_option::{StaticOption, StaticResult};
	/// let option = StaticOption::<StaticResult<i32, &'static str, false>, false>::none();
	/// assert_eq!(StaticResult::new_ok(StaticOption::none()), option.transpose());
	/// ```
	pub const fn transpose(self) -> StaticResult<StaticOption<T, false>, E, true> {
		StaticResult::new_ok(StaticOption::none())
	}
}

impl<T, const IS_SOME: bool> StaticOption<T, IS_SOME> {
	/// See [`core::option::Option::is_some`].
	///
	/// Return `true` if this [`StaticOption`] contains a value, `false` otherwise.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	///	let some = StaticOption::some(42);
	/// assert!(some.is_some());
	/// let none = StaticOption::<i32, false>::none();
	/// assert!(!none.is_some());
	/// ```
	pub const fn is_some(&self) -> bool {
		IS_SOME
	}

	/// See [`core::option::Option::is_none`].
	///
	/// Return `false` if this [`StaticOption`] contains a value, `true` otherwise.
	///
	/// # Example
	/// ```
	/// # use static_option::StaticOption;
	/// let none = StaticOption::<i32, false>::none();
	/// assert!(none.is_none());
	///	let some = StaticOption::some(42);
	/// assert!(!some.is_none());
	/// ```
	pub const fn is_none(&self) -> bool {
		!IS_SOME
	}

	/// See [`core::option::Option::as_ref`].
	///
	/// Given a reference to a [`StaticOption`], returns an owned [`StaticOption`] containing a reference
	/// to the value in the referenced [`StaticOption`].
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::some(42);
	/// assert_eq!(StaticOption::some(&42), option.as_ref());
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let option = StaticOption::<i32, false>::none();
	/// assert_eq!(StaticOption::<&i32, false>::none(), option.as_ref());
	/// ```
	pub fn as_ref(&self) -> StaticOption<&T, IS_SOME> {
		StaticOption {
			value: match self.as_option() {
				Some(value) => MaybeUninit::new(value),
				None => MaybeUninit::uninit(),
			},
		}
	}

	/// See [`core::option::Option::as_ref`].
	///
	/// Given a mutable reference to a [`StaticOption`], returns an owned [`StaticOption`] containing a mutable reference
	/// to the value in the referenced [`StaticOption`].
	///
	/// # Examples
	/// ```
	/// # use static_option::StaticOption;
	/// let mut option = StaticOption::some(42);
	/// let referencing = option.as_mut();
	/// assert_eq!(StaticOption::some(&mut 42), referencing);
	/// *referencing.into_inner() = 1337;
	/// assert_eq!(StaticOption::some(1337), option);
	/// ```
	///
	/// ```
	/// # use static_option::StaticOption;
	/// let mut option = StaticOption::<i32, false>::none();
	/// assert_eq!(StaticOption::<&mut i32, false>::none(), option.as_mut());
	/// ```
	pub fn as_mut(&mut self) -> StaticOption<&mut T, IS_SOME> {
		StaticOption {
			value: match self.as_mut_option() {
				Some(value) => MaybeUninit::new(value),
				None => MaybeUninit::uninit(),
			},
		}
	}

	pub fn as_pin_ref(self: Pin<&Self>) -> StaticOption<Pin<&T>, IS_SOME> {
		StaticOption {
			// SAFETY: self.get_ref() is guaranteed to be pinned because it comes from `self`
			// which is pinned
			value: match self.get_ref().as_option() {
				Some(value) => MaybeUninit::new(unsafe { Pin::new_unchecked(value) }),
				None => MaybeUninit::uninit(),
			},
		}
	}

	pub fn as_pin_mut(self: Pin<&mut Self>) -> StaticOption<Pin<&mut T>, IS_SOME> {
		StaticOption {
			// SAFETY: self.get_mut_unchecked() is guaranteed to be pinned because it comes from `self`
			// which is pinned and it will be repinned again.
			value: match unsafe { self.get_unchecked_mut() }.as_mut_option() {
				Some(value) => MaybeUninit::new(unsafe { Pin::new_unchecked(value) }),
				None => MaybeUninit::uninit(),
			},
		}
	}

	pub fn ok_or<E>(self, error: E) -> StaticResult<T, E, IS_SOME> {
		match self.into_option() {
			Some(value) => StaticResult {
				ok: ManuallyDrop::new(value),
			},
			None => StaticResult {
				error: ManuallyDrop::new(error),
			},
		}
	}

	pub fn ok_or_else<E, F>(self, error: F) -> StaticResult<T, E, IS_SOME>
	where
		F: FnOnce() -> E,
	{
		match self.into_option() {
			Some(value) => StaticResult {
				ok: ManuallyDrop::new(value),
			},
			None => StaticResult {
				error: ManuallyDrop::new(error()),
			},
		}
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		match self.into_option() {
			Some(value) => value,
			None => T::default(),
		}
	}

	pub fn expect(self, message: &str) -> T {
		self.into_option().expect(message)
	}

	pub fn unwrap(self) -> T {
		match self.into_option() {
			Some(value) => value,
			None => {
				panic!("called `StaticOption::unwrap()` on a `None` value")
			}
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

	pub fn as_deref(&self) -> StaticOption<&<T as Deref>::Target, IS_SOME>
	where
		T: Deref,
	{
		StaticOption {
			value: match self.as_option() {
				Some(value) => MaybeUninit::new(value.deref()),
				None => MaybeUninit::uninit(),
			},
		}
	}

	pub fn as_deref_mut(&mut self) -> StaticOption<&mut <T as Deref>::Target, IS_SOME>
	where
		T: DerefMut,
	{
		StaticOption {
			value: match self.as_mut_option() {
				Some(value) => MaybeUninit::new(value.deref_mut()),
				None => MaybeUninit::uninit(),
			},
		}
	}

	pub fn map<U, F>(self, f: F) -> StaticOption<U, IS_SOME>
	where
		F: FnOnce(T) -> U,
	{
		StaticOption {
			value: self
				.into_option()
				.map(f)
				.map_or_else(MaybeUninit::uninit, MaybeUninit::new),
		}
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

	pub fn iter(&self) -> Iter<&T> {
		self.as_ref().into_iter()
	}

	pub fn iter_mut(&mut self) -> Iter<&mut T> {
		self.as_mut().into_iter()
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

impl<T, const IS_SOME: bool> Clone for StaticOption<T, IS_SOME>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		self.as_ref().cloned()
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

impl<'a, T, const IS_SOME: bool> From<&'a StaticOption<T, IS_SOME>> for StaticOption<&'a T, IS_SOME> {
	fn from(static_option: &'a StaticOption<T, IS_SOME>) -> Self {
		static_option.as_ref()
	}
}

impl<'a, T, const IS_SOME: bool> From<&'a mut StaticOption<T, IS_SOME>> for StaticOption<&'a mut T, IS_SOME> {
	fn from(static_option: &'a mut StaticOption<T, IS_SOME>) -> Self {
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
	type IntoIter = Iter<T>;

	fn into_iter(self) -> Self::IntoIter {
		Iter::new(self.into_option())
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

impl<T, const IS_SOME: bool> Copy for StaticOption<T, IS_SOME> where T: Copy {}
