use crate::iterator::Iter;
use crate::StaticResult;
use core::any::type_name;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::mem::{swap, ManuallyDrop};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;

// A union is used instead of `MaybeUninit` because `assume_init` isn't a const fn in Rust 1.56, but union fields *can* be accessed inside a const fn.
#[must_use = "Call `.drop()` if you don't use the StaticOption, otherwise it's contents never get dropped."]
pub union StaticOption<T, const IS_SOME: bool> {
	some: ManuallyDrop<T>,
	none: (),
}

impl<T> StaticOption<T, true> {
	/// Create a [`StaticOption<T, true>`] with a value inside. The `true` type parameter statically tracks
	/// the fact that a value is inside.
	pub const fn some(value: T) -> Self {
		StaticOption::new_some(value)
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
	pub const fn into_inner(self) -> T {
		self.inner()
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
		self.as_inner()
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
		self.as_inner_mut()
	}

	/// See [`core::option::Option::and`].
	///
	/// Return `option_b`, dropping `self`.
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
	pub fn and<U, const IS_SOME: bool>(self, option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, IS_SOME> {
		self.drop();
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
	/// Return `self`, dropping `option_b`.
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
	pub fn or<const IS_SOME: bool>(self, option_b: StaticOption<T, IS_SOME>) -> Self {
		option_b.drop();
		self
	}

	/// See [`core::option::Option::or_else`].
	///
	/// Return `self`, ignoring `_fallback`.
	///
	/// Warning: Since `_fallback` is ignored, any captured `StaticOption` will not be dropped.
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
		Self { none: () }
	}

	/// See [`core::option::Option::and`].
	///
	/// Return [`StaticOption<U, false>::none()`], dropping `option_b`.
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
	pub fn and<U, const IS_SOME: bool>(self, option_b: StaticOption<U, IS_SOME>) -> StaticOption<U, false> {
		option_b.drop();
		StaticOption::none()
	}

	/// See [`core::option::Option::and_then`].
	///
	/// Return [`StaticOption<U, false>::none()`], ignoring `_mapper`.
	/// Warning: Since `_mapper` is ignored, any captured `StaticOption` will not be dropped.
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
		// self doesn't need to get dropped since it is none
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
		// self doesn't need to be dropped since it is none
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
		if IS_SOME {
			StaticOption::new_some(*self.inner())
		} else {
			StaticOption::new_none()
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
		if IS_SOME {
			StaticOption::new_some(self.inner().clone())
		} else {
			StaticOption::new_none()
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
	pub const fn flatten(self) -> StaticOption<T, IS_SOME> {
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
		// self doesn't need to be dropped since it is none
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
	pub const fn transpose(self) -> StaticResult<StaticOption<T, true>, E, IS_OK> {
		let result = self.into_inner();
		if IS_OK {
			StaticResult::create_ok(StaticOption::new_some(result.inner_ok()))
		} else {
			StaticResult::create_err(result.inner_error())
		}
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
		// self doesn't need to be dropped since it is none
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
		if IS_SOME {
			StaticOption::new_some(self.as_inner())
		} else {
			StaticOption::new_none()
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
		if IS_SOME {
			StaticOption::new_some(self.as_inner_mut())
		} else {
			StaticOption::new_none()
		}
	}

	pub fn as_pin_ref(self: Pin<&Self>) -> StaticOption<Pin<&T>, IS_SOME> {
		if IS_SOME {
			// SAFETY: self.get_ref() is guaranteed to be pinned because it comes from `self` which is pinned
			StaticOption::new_some(unsafe { Pin::new_unchecked(self.get_ref().as_inner()) })
		} else {
			StaticOption::new_none()
		}
	}

	pub fn as_pin_mut(self: Pin<&mut Self>) -> StaticOption<Pin<&mut T>, IS_SOME> {
		if IS_SOME {
			// SAFETY: self.get_mut_unchecked() is guaranteed to be pinned because it comes from `self`
			// which is pinned and it will be repinned again.
			StaticOption::new_some(unsafe { Pin::new_unchecked(self.get_unchecked_mut().as_inner_mut()) })
		} else {
			StaticOption::new_none()
		}
	}

	pub fn ok_or<E>(self, error: E) -> StaticResult<T, E, IS_SOME> {
		if IS_SOME {
			StaticResult::create_ok(self.inner())
		} else {
			StaticResult::create_err(error)
		}
	}

	pub fn ok_or_else<E, F>(self, error: F) -> StaticResult<T, E, IS_SOME>
	where
		F: FnOnce() -> E,
	{
		if IS_SOME {
			StaticResult::create_ok(self.inner())
		} else {
			StaticResult::create_err(error())
		}
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		if IS_SOME {
			self.inner()
		} else {
			T::default()
		}
	}

	pub fn expect(self, message: &str) -> T {
		if IS_SOME {
			self.inner()
		} else {
			panic!("{}", message)
		}
	}

	pub fn unwrap(self) -> T {
		if IS_SOME {
			self.inner()
		} else {
			panic!("called `unwrap()` on {}", type_name::<Self>())
		}
	}

	pub fn unwrap_or(self, default: T) -> T {
		if IS_SOME {
			self.inner()
		} else {
			default
		}
	}

	pub fn unwrap_or_else<F>(self, function: F) -> T
	where
		F: FnOnce() -> T,
	{
		if IS_SOME {
			self.inner()
		} else {
			function()
		}
	}

	pub fn as_deref(&self) -> StaticOption<&<T as Deref>::Target, IS_SOME>
	where
		T: Deref,
	{
		if IS_SOME {
			StaticOption::new_some(self.as_inner())
		} else {
			StaticOption::new_none()
		}
	}

	pub fn as_deref_mut(&mut self) -> StaticOption<&mut <T as Deref>::Target, IS_SOME>
	where
		T: DerefMut,
	{
		if IS_SOME {
			StaticOption::new_some(self.as_inner_mut())
		} else {
			StaticOption::new_none()
		}
	}

	pub fn map<U, F>(self, mapper: F) -> StaticOption<U, IS_SOME>
	where
		F: FnOnce(T) -> U,
	{
		if IS_SOME {
			StaticOption::new_some(mapper(self.inner()))
		} else {
			StaticOption::new_none()
		}
	}

	pub fn map_or<U, F>(self, default: U, mapper: F) -> U
	where
		F: FnOnce(T) -> U,
	{
		if IS_SOME {
			mapper(self.inner())
		} else {
			default
		}
	}

	pub fn map_or_else<U, D, F>(self, default: D, mapper: F) -> U
	where
		F: FnOnce(T) -> U,
		D: FnOnce() -> U,
	{
		if IS_SOME {
			mapper(self.inner())
		} else {
			default()
		}
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
			unsafe { ManuallyDrop::drop(&mut self.some) }
		}
	}

	pub const fn into_option(self) -> Option<T> {
		if IS_SOME {
			Some(self.inner())
		} else {
			None
		}
	}

	pub fn as_option(&self) -> Option<&T> {
		if IS_SOME {
			Some(self.as_inner())
		} else {
			None
		}
	}

	pub fn as_mut_option(&mut self) -> Option<&mut T> {
		if IS_SOME {
			Some(self.as_inner_mut())
		} else {
			None
		}
	}

	// Equivalent to `some` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) const fn new_some(value: T) -> Self {
		// SAFETY: The assert ensures that only `StaticOption<T, true>` are constructed like this.
		assert!(IS_SOME); // gets optimized away
		Self {
			some: ManuallyDrop::new(value),
		}
	}

	// Equivalent to `none` but doesn't require explicit `false` as type parameter.
	#[inline(always)]
	pub(crate) const fn new_none() -> Self {
		// SAFETY: The assert ensures that only `StaticOption<T, false>` are constructed like this.
		assert!(!IS_SOME); // gets optimized away
		Self { none: () }
	}

	// Equivalent to `into_inner` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) const fn inner(self) -> T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		// and the assert ensures that the `some` union field is only accessed when it is initialized
		assert!(IS_SOME); // gets optimized away
		ManuallyDrop::into_inner(unsafe { self.some })
	}

	// Equivalent to `inner_ref` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) fn as_inner(&self) -> &T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		// and the assert ensures that the `some` union field is only accessed when it is initialized
		assert!(IS_SOME); // gets optimized away
		unsafe { &self.some }
	}

	// Equivalent to `inner_mut` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) fn as_inner_mut(&mut self) -> &mut T {
		// SAFETY: StaticOption<T, true> can only be constructed with a value inside (tracked by the `true`)
		// and the assert ensures that the `some` union field is only accessed when it is initialized
		assert!(IS_SOME); // gets optimized away
		unsafe { &mut self.some }
	}
}

impl<T> Default for StaticOption<T, false> {
	fn default() -> Self {
		StaticOption::new_none()
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
		if IS_SOME {
			formatter
				.debug_tuple("StaticOption::some")
				.field(self.as_inner())
				.finish()
		} else {
			formatter.debug_tuple("StaticOption::none").finish()
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
