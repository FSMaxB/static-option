use core::mem::ManuallyDrop;

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

	pub fn get_ok(self) -> T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { ManuallyDrop::into_inner(self.ok) }
	}

	pub fn get_ok_ref(&self) -> &T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { &self.ok }
	}

	pub fn get_ok_mut(&mut self) -> &mut T {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { &mut self.ok }
	}

	pub fn drop(mut self) {
		// SAFETY: StaticResult<T, E, true> can only be constructed with ok value inside (tracked by the true)
		unsafe { ManuallyDrop::drop(&mut self.ok) }
	}
}

impl<T, E> StaticResult<T, E, false> {
	pub fn err(error: E) -> StaticResult<T, E, false> {
		Self {
			error: ManuallyDrop::new(error),
		}
	}

	pub fn get_err(self) -> E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { ManuallyDrop::into_inner(self.error) }
	}

	pub fn get_err_ref(&self) -> &E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { &self.error }
	}

	pub fn get_err_mut(&mut self) -> &mut E {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { &mut self.error }
	}

	pub fn drop(mut self) {
		// SAFETY: StaticResult<T, E, false> can only be constructed with error value inside (tracked by the false)
		unsafe { ManuallyDrop::drop(&mut self.error) }
	}
}

impl<T, E> From<StaticResult<T, E, false>> for Result<T, E> {
	fn from(static_result: StaticResult<T, E, false>) -> Self {
		Err(static_result.get_err())
	}
}

impl<T, E> From<StaticResult<T, E, true>> for Result<T, E> {
	fn from(static_result: StaticResult<T, E, true>) -> Self {
		Ok(static_result.get_ok())
	}
}
