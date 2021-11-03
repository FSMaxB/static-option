use crate::StaticOption;
use core::cmp::Ordering;
use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::result::IntoIter;

#[must_use = "Call `.drop()` if you don't use the `StaticResult`, otherwise it's contents never get dropped."]
pub union StaticResult<T, E, const IS_OK: bool> {
	ok: ManuallyDrop<T>,
	error: ManuallyDrop<E>,
}

impl<T, E> StaticResult<T, E, true> {
	pub fn new_ok(ok: T) -> StaticResult<T, E, true> {
		Self {
			ok: ManuallyDrop::new(ok),
		}
	}

	pub fn ok(self) -> StaticOption<T, true> {
		StaticOption::some(self.into_ok())
	}

	pub fn err(self) -> StaticOption<E, false> {
		StaticOption::none()
	}

	pub fn as_ref(&self) -> StaticResult<&T, &E, true> {
		StaticResult::new_ok(self.ok_ref())
	}

	pub fn as_mut(&mut self) -> StaticResult<&mut T, &mut E, true> {
		StaticResult::new_ok(self.ok_mut())
	}

	pub fn map_err<F, O>(self, _op: O) -> StaticResult<T, F, true>
	where
		O: FnOnce(E) -> F,
	{
		StaticResult::new_ok(self.into_ok())
	}

	pub fn and<U, const IS_SOME: bool>(self, res: StaticResult<U, E, IS_SOME>) -> StaticResult<U, E, IS_SOME> {
		res
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, op: F) -> StaticResult<U, E, IS_SOME>
	where
		F: FnOnce(T) -> StaticResult<U, E, IS_SOME>,
	{
		op(self.into_ok())
	}

	pub fn or<F, const IS_SOME: bool>(self, _res: StaticResult<T, F, IS_SOME>) -> StaticResult<T, F, true> {
		StaticResult::new_ok(self.into_ok())
	}

	pub fn or_else<F, O, const IS_SOME: bool>(self, _op: O) -> StaticResult<T, F, true>
	where
		O: FnOnce(E) -> StaticResult<T, F, IS_SOME>,
	{
		StaticResult::new_ok(self.into_ok())
	}

	pub fn as_deref(&self) -> StaticResult<&<T as Deref>::Target, &E, true>
	where
		T: Deref,
	{
		StaticResult::new_ok(self.ok_ref().deref())
	}

	pub fn as_deref_mut(&mut self) -> StaticResult<&mut <T as Deref>::Target, &E, true>
	where
		T: DerefMut,
	{
		StaticResult::new_ok(self.ok_mut().deref_mut())
	}

	pub fn into_ok(self) -> T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		// and it's insides are never dropped without dropping the entire StaticResult
		unsafe { ManuallyDrop::into_inner(self.ok) }
	}

	pub fn ok_ref(&self) -> &T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { &self.ok }
	}

	pub fn ok_mut(&mut self) -> &mut T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { &mut self.ok }
	}
}

impl<T, E, const IS_SOME: bool> StaticResult<StaticOption<T, IS_SOME>, E, true> {
	pub fn transpose(self) -> StaticOption<StaticResult<T, E, true>, IS_SOME> {
		let option = self.into_ok();
		option.map(StaticResult::new_ok)
	}
}

impl<T, E, const IS_SOME: bool> StaticResult<StaticOption<T, IS_SOME>, E, false> {
	pub fn transpose(self) -> StaticOption<StaticResult<T, E, false>, true> {
		StaticOption::some(StaticResult::new_err(self.into_err()))
	}
}

impl<T, E> StaticResult<T, E, false> {
	pub fn new_err(error: E) -> StaticResult<T, E, false> {
		Self {
			error: ManuallyDrop::new(error),
		}
	}

	pub fn ok(self) -> StaticOption<T, false> {
		StaticOption::none()
	}

	pub fn err(self) -> StaticOption<E, true> {
		StaticOption::some(self.into_err())
	}

	pub fn as_ref(&self) -> StaticResult<&T, &E, false> {
		StaticResult::new_err(self.err_ref())
	}

	pub fn as_mut(&mut self) -> StaticResult<&mut T, &mut E, false> {
		StaticResult::new_err(self.err_mut())
	}

	pub fn map_err<F, O>(self, op: O) -> StaticResult<T, F, false>
	where
		O: FnOnce(E) -> F,
	{
		StaticResult::new_err(op(self.into_err()))
	}

	pub fn and<U, const IS_SOME: bool>(self, _res: StaticResult<U, E, IS_SOME>) -> StaticResult<U, E, false> {
		StaticResult::new_err(self.into_err())
	}

	pub fn and_then<U, F, const IS_SOME: bool>(self, _op: F) -> StaticResult<U, E, false>
	where
		F: FnOnce(T) -> StaticResult<U, E, IS_SOME>,
	{
		StaticResult::new_err(self.into_err())
	}

	pub fn or<F, const IS_SOME: bool>(self, res: StaticResult<T, F, IS_SOME>) -> StaticResult<T, F, IS_SOME> {
		res
	}

	pub fn or_else<F, O, const IS_SOME: bool>(self, op: O) -> StaticResult<T, F, IS_SOME>
	where
		O: FnOnce(E) -> StaticResult<T, F, IS_SOME>,
	{
		op(self.into_err())
	}

	pub fn as_deref(&self) -> StaticResult<&<T as Deref>::Target, &E, false>
	where
		T: Deref,
	{
		StaticResult::new_err(self.err_ref())
	}

	pub fn as_deref_mut(&mut self) -> StaticResult<&mut <T as Deref>::Target, &E, false>
	where
		T: DerefMut,
	{
		StaticResult::new_err(self.err_mut())
	}

	pub fn into_err(self) -> E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		// and it's insides are never dropped without dropping the entire StaticResult
		unsafe { ManuallyDrop::into_inner(self.error) }
	}

	pub fn err_ref(&self) -> &E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { &self.error }
	}

	pub fn err_mut(&mut self) -> &mut E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { &mut self.error }
	}
}

impl<T, E, const IS_OK: bool> StaticResult<T, E, IS_OK> {
	pub const fn is_ok(&self) -> bool {
		IS_OK
	}

	pub const fn is_err(&self) -> bool {
		!IS_OK
	}

	pub fn map<U, F>(self, op: F) -> StaticResult<U, E, IS_OK>
	where
		F: FnOnce(T) -> U,
	{
		match self.into_result().map(op) {
			Ok(ok) => StaticResult {
				ok: ManuallyDrop::new(ok),
			},
			Err(error) => StaticResult {
				error: ManuallyDrop::new(error),
			},
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

	// TODO: Implement .iter() and .iter_mut()

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

	pub fn into_result(self) -> Result<T, E> {
		if IS_OK {
			// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
			Ok(ManuallyDrop::into_inner(unsafe { self.ok }))
		} else {
			// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
			Err(ManuallyDrop::into_inner(unsafe { self.error }))
		}
	}

	pub fn as_result(&self) -> Result<&T, &E> {
		if IS_OK {
			// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
			Ok(unsafe { self.ok.deref() })
		} else {
			// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
			Err(unsafe { self.error.deref() })
		}
	}
	pub fn as_mut_result(&mut self) -> Result<&mut T, &mut E> {
		if IS_OK {
			// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
			Ok(unsafe { self.ok.deref_mut() })
		} else {
			// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
			Err(unsafe { self.error.deref_mut() })
		}
	}
}

impl<T, E, const IS_OK: bool> Clone for StaticResult<T, E, IS_OK>
where
	T: Clone,
	E: Clone,
{
	fn clone(&self) -> Self {
		match self.as_result() {
			Ok(ok) => Self {
				ok: ManuallyDrop::new(ok.clone()),
			},
			Err(error) => Self {
				error: ManuallyDrop::new(error.clone()),
			},
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
	type IntoIter = IntoIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.into_result().into_iter()
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

impl<T, E> From<StaticResult<T, E, false>> for Result<T, E> {
	fn from(static_result: StaticResult<T, E, false>) -> Self {
		Err(static_result.into_err())
	}
}

impl<T, E> From<StaticResult<T, E, true>> for Result<T, E> {
	fn from(static_result: StaticResult<T, E, true>) -> Self {
		Ok(static_result.into_ok())
	}
}
