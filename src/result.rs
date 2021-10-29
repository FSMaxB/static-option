use core::mem::ManuallyDrop;

#[must_use = "Call `.drop()` if you don't use the `StaticResult`, otherwise it's contents never get dropped."]
pub union StaticResult<T, E, const STATE: bool> {
	ok: ManuallyDrop<T>,
	error: ManuallyDrop<E>,
}

impl<T, E> StaticResult<T, E, true> {
	pub fn ok(ok: T) -> StaticResult<T, E, true> {
		Self {
			ok: ManuallyDrop::new(ok),
		}
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

impl<T, E> StaticResult<T, E, false> {
	pub fn err(error: E) -> StaticResult<T, E, false> {
		Self {
			error: ManuallyDrop::new(error),
		}
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

impl<T, E, const STATE: bool> StaticResult<T, E, STATE> {
	pub fn drop(mut self) {
		if STATE {
			// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
			// and it's insides are never dropped without dropping the entire StaticResult (happening here)
			unsafe { ManuallyDrop::drop(&mut self.ok) }
		} else {
			// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
			// and it's insides are never dropped without dropping the entire StaticResult (happening here)
			unsafe { ManuallyDrop::drop(&mut self.error) }
		}
	}
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
