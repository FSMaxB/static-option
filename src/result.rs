use crate::{const_assert, Iter, StaticOption};
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

#[must_use = "Call `.drop()` if you don't use the `StaticResult`, otherwise it's contents never get dropped."]
pub union StaticResult<T, E, const IS_OK: bool> {
	pub(crate) ok: ManuallyDrop<T>,
	pub(crate) error: ManuallyDrop<E>,
}

impl<T, E> StaticResult<T, E, true> {
	pub const fn new_ok(ok: T) -> StaticResult<T, E, true> {
		StaticResult::create_ok(ok)
	}

	pub fn and<U, const IS_SOME: bool>(self, res: StaticResult<U, E, IS_SOME>) -> StaticResult<U, E, IS_SOME> {
		self.drop();
		res
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, op: F) -> StaticResult<U, E, IS_SOME>
	where
		F: FnOnce(T) -> StaticResult<U, E, IS_SOME>,
	{
		op(self.into_ok())
	}

	pub fn or<F, const IS_SOME: bool>(self, res: StaticResult<T, F, IS_SOME>) -> StaticResult<T, F, true> {
		res.drop();
		StaticResult::new_ok(self.into_ok())
	}

	pub fn or_else<F, O, const IS_SOME: bool>(self, _op: O) -> StaticResult<T, F, true>
	where
		O: FnOnce(E) -> StaticResult<T, F, IS_SOME>,
	{
		StaticResult::new_ok(self.into_ok())
	}

	pub const fn into_ok(self) -> T {
		self.inner_ok()
	}

	pub fn ok_ref(&self) -> &T {
		self.as_ok()
	}

	pub fn ok_mut(&mut self) -> &mut T {
		self.as_ok_mut()
	}
}

impl<T, E, const IS_SOME: bool> StaticResult<StaticOption<T, IS_SOME>, E, true> {
	pub fn transpose(self) -> StaticOption<StaticResult<T, E, true>, IS_SOME> {
		let option = self.into_ok();
		if IS_SOME {
			StaticOption::new_some(StaticResult::create_ok(option.inner()))
		} else {
			option.drop();
			StaticOption::new_none()
		}
	}
}

impl<T, E, const IS_SOME: bool> StaticResult<StaticOption<T, IS_SOME>, E, false> {
	pub const fn transpose(self) -> StaticOption<StaticResult<T, E, false>, true> {
		StaticOption::some(StaticResult::new_err(self.into_err()))
	}
}

impl<T, E> StaticResult<T, E, false> {
	pub const fn new_err(error: E) -> StaticResult<T, E, false> {
		StaticResult::create_err(error)
	}

	pub fn and<U, const IS_SOME: bool>(self, res: StaticResult<U, E, IS_SOME>) -> StaticResult<U, E, false> {
		res.drop();
		StaticResult::new_err(self.into_err())
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, _op: F) -> StaticResult<U, E, false>
	where
		F: FnOnce(T) -> StaticResult<U, E, IS_SOME>,
	{
		StaticResult::new_err(self.into_err())
	}

	pub fn or<F, const IS_SOME: bool>(self, res: StaticResult<T, F, IS_SOME>) -> StaticResult<T, F, IS_SOME> {
		self.drop();
		res
	}

	pub fn or_else<F, O, const IS_SOME: bool>(self, op: O) -> StaticResult<T, F, IS_SOME>
	where
		O: FnOnce(E) -> StaticResult<T, F, IS_SOME>,
	{
		op(self.into_err())
	}

	pub const fn into_err(self) -> E {
		self.inner_error()
	}

	pub fn err_ref(&self) -> &E {
		self.as_error()
	}

	pub fn err_mut(&mut self) -> &mut E {
		self.as_error_mut()
	}
}

impl<T, E, const IS_OK: bool> StaticResult<T, E, IS_OK> {
	pub const fn is_ok(&self) -> bool {
		IS_OK
	}

	pub const fn is_err(&self) -> bool {
		!IS_OK
	}

	pub const fn ok(self) -> StaticOption<T, IS_OK> {
		if IS_OK {
			StaticOption::new_some(self.inner_ok())
		} else {
			StaticOption::new_none()
		}
	}

	pub fn err(self) -> StaticOption<E, true> {
		if IS_OK {
			self.drop();
			StaticOption::new_none()
		} else {
			StaticOption::new_some(self.inner_error())
		}
	}

	pub fn as_ref(&self) -> StaticResult<&T, &E, IS_OK> {
		match self.as_result() {
			Ok(ok) => StaticResult::create_ok(ok),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn as_mut(&mut self) -> StaticResult<&mut T, &mut E, IS_OK> {
		match self.as_mut_result() {
			Ok(ok) => StaticResult::create_ok(ok),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn map_err<F, O>(self, op: O) -> StaticResult<T, F, IS_OK>
	where
		O: FnOnce(E) -> F,
	{
		match self.into_result().map_err(op) {
			Ok(ok) => StaticResult::create_ok(ok),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn as_deref(&self) -> StaticResult<&<T as Deref>::Target, &E, IS_OK>
	where
		T: Deref,
	{
		match self.as_result() {
			Ok(ok) => StaticResult::create_ok(ok.deref()),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn as_deref_mut(&mut self) -> StaticResult<&mut <T as Deref>::Target, &E, IS_OK>
	where
		T: DerefMut,
	{
		match self.as_mut_result() {
			Ok(ok) => StaticResult::create_ok(ok.deref_mut()),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn map<U, F>(self, op: F) -> StaticResult<U, E, IS_OK>
	where
		F: FnOnce(T) -> U,
	{
		match self.into_result().map(op) {
			Ok(ok) => StaticResult::create_ok(ok),
			Err(error) => StaticResult::create_err(error),
		}
	}

	pub fn map_or<U, F>(self, default: U, f: F) -> U
	where
		F: FnOnce(T) -> U,
	{
		self.into_result().map_or(default, f)
	}

	pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
	where
		F: FnOnce(T) -> U,
		D: FnOnce(E) -> U,
	{
		self.into_result().map_or_else(default, f)
	}

	pub fn iter(&self) -> Iter<&T> {
		self.as_ref().ok().into_iter()
	}

	pub fn iter_mut(&mut self) -> Iter<&mut T> {
		self.as_mut().ok().into_iter()
	}

	pub fn unwrap_or(self, default: T) -> T {
		self.into_result().unwrap_or(default)
	}

	pub fn unwrap_or_else<F>(self, op: F) -> T
	where
		F: FnOnce(E) -> T,
	{
		self.into_result().unwrap_or_else(op)
	}

	pub fn expect(self, msg: &str) -> T
	where
		E: Debug,
	{
		self.into_result().expect(msg)
	}

	pub fn unwrap(self) -> T
	where
		E: Debug,
	{
		// TODO: Update panic error message to not contain `core::result::Result`
		self.into_result().unwrap()
	}

	pub fn expect_err(self, msg: &str) -> E
	where
		T: Debug,
	{
		self.into_result().expect_err(msg)
	}

	pub fn unwrap_err(self) -> E
	where
		T: Debug,
	{
		// TODO: Update panic error message to not contain `core::result::Result`
		self.into_result().unwrap_err()
	}

	pub fn unwrap_or_default(self) -> T
	where
		T: Default,
	{
		self.into_result().unwrap_or_default()
	}

	pub fn drop(mut self) {
		if IS_OK {
			// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
			// and it's insides are never dropped without dropping the entire StaticResult (happening here)
			unsafe { ManuallyDrop::drop(&mut self.ok) }
		} else {
			// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
			// and it's insides are never dropped without dropping the entire StaticResult (happening here)
			unsafe { ManuallyDrop::drop(&mut self.error) }
		}
	}

	pub const fn into_result(self) -> Result<T, E> {
		if IS_OK {
			Ok(self.inner_ok())
		} else {
			Err(self.inner_error())
		}
	}

	pub fn as_result(&self) -> Result<&T, &E> {
		if IS_OK {
			Ok(self.as_ok())
		} else {
			Err(self.as_error())
		}
	}

	pub fn as_mut_result(&mut self) -> Result<&mut T, &mut E> {
		if IS_OK {
			Ok(self.as_ok_mut())
		} else {
			Err(self.as_error_mut())
		}
	}

	// Equivalent to `new_ok` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) const fn create_ok(ok: T) -> Self {
		// SAFETY: The const_assert ensures that only `StaticResult<T, E, true>` are constructed like this.
		const_assert(IS_OK); // gets optimized away
		Self {
			ok: ManuallyDrop::new(ok),
		}
	}

	// Equivalent to `new_err` but doesn't require explicit `false` as type parameter.
	#[inline(always)]
	pub(crate) const fn create_err(error: E) -> Self {
		// SAFETY: The const_assert ensures that only `StaticResult<T, E, true>` are constructed like this.
		const_assert(!IS_OK); // gets optimized away
		Self {
			error: ManuallyDrop::new(error),
		}
	}

	// Equivalent to `into_ok` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) const fn inner_ok(self) -> T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with a value inside (tracked by the `true`)
		// and the const_assert ensures that the `ok` union field is only accessed when it is initialized
		const_assert(IS_OK); // gets optimized away
		ManuallyDrop::into_inner(unsafe { self.ok })
	}

	// Equivalent to `ok_ref` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) fn as_ok(&self) -> &T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with a value inside (tracked by the `true`)
		// and the assert ensures that the `ok` union field is only accessed when it is initialized
		assert!(IS_OK); // gets optimized away
		unsafe { &self.ok }
	}

	// Equivalent to `ok_mut` but doesn't require explicit `true` as type parameter.
	#[inline(always)]
	pub(crate) fn as_ok_mut(&mut self) -> &mut T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with a value inside (tracked by the `true`)
		// and the assert ensures that the `ok` union field is only accessed when it is initialized
		assert!(IS_OK); // gets optimized away
		unsafe { &mut self.ok }
	}

	// Equivalent to `into_err` but doesn't require explicit `false` as type parameter.
	#[inline(always)]
	pub(crate) const fn inner_error(self) -> E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with a value inside (tracked by the `false`)
		// and the const_assert ensures that the `error` union field is only accessed when it is initialized
		const_assert(!IS_OK); // gets optimized away
		ManuallyDrop::into_inner(unsafe { self.error })
	}

	// Equivalent to `err_ref` but doesn't require explicit `false` as type parameter.
	#[inline(always)]
	pub(crate) fn as_error(&self) -> &E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with a value inside (tracked by the `false`)
		// and the assert ensures that the `error` union field is only accessed when it is initialized
		assert!(!IS_OK);
		unsafe { &self.error }
	}

	// Equivalent to `err_mut` but doesn't require explicit `false` as type parameter.
	#[inline(always)]
	pub(crate) fn as_error_mut(&mut self) -> &mut E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with a value inside (tracked by the `false`)
		// and the assert ensures that the `error` union field is only accessed when it is initialized
		assert!(!IS_OK);
		unsafe { &mut self.error }
	}
}

impl<T, E, const IS_OK: bool> Clone for StaticResult<T, E, IS_OK>
where
	T: Clone,
	E: Clone,
{
	fn clone(&self) -> Self {
		match self.as_result() {
			Ok(ok) => StaticResult::create_ok(ok.clone()),
			Err(error) => StaticResult::create_err(error.clone()),
		}
	}
}

impl<T, E, const IS_OK: bool> Hash for StaticResult<T, E, IS_OK>
where
	T: Hash,
	E: Hash,
{
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self.as_result() {
			Ok(ok) => ok.hash(state),
			Err(error) => error.hash(state),
		}
	}
}

impl<T, E, const IS_OK: bool> IntoIterator for StaticResult<T, E, IS_OK> {
	type Item = T;
	type IntoIter = Iter<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.ok().into_iter()
	}
}

impl<T, E, const IS_OK: bool> PartialEq for StaticResult<T, E, IS_OK>
where
	T: PartialEq,
	E: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.as_result().eq(&other.as_result())
	}
}

impl<T, E, const IS_OK: bool> Eq for StaticResult<T, E, IS_OK>
where
	T: Eq,
	E: Eq,
{
}

impl<T, E, const IS_OK: bool> PartialOrd for StaticResult<T, E, IS_OK>
where
	T: PartialOrd,
	E: PartialOrd,
{
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.as_result().partial_cmp(&other.as_result())
	}
}

impl<T, E, const IS_OK: bool> Ord for StaticResult<T, E, IS_OK>
where
	T: Ord,
	E: Ord,
{
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_result().cmp(&other.as_result())
	}
}

impl<T, E, const IS_OK: bool> Copy for StaticResult<T, E, IS_OK>
where
	T: Copy,
	E: Copy,
{
}

impl<T, E, const IS_OK: bool> From<StaticResult<T, E, IS_OK>> for Result<T, E> {
	fn from(static_result: StaticResult<T, E, IS_OK>) -> Self {
		static_result.into_result()
	}
}

impl<T, E, const IS_OK: bool> Debug for StaticResult<T, E, IS_OK>
where
	T: Debug,
	E: Debug,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
		match self.as_result() {
			Ok(ok) => formatter.debug_tuple("StaticResult::ok").field(ok).finish(),
			Err(error) => formatter.debug_tuple("StaticOption::err").field(error).finish(),
		}
	}
}
