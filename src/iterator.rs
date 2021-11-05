pub struct Iter<T> {
	value: Option<T>,
}

impl<T> Iter<T> {
	pub(crate) const fn new(value: Option<T>) -> Self {
		Self { value }
	}
}

impl<T> Iterator for Iter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.value.take()
	}
}
