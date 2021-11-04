# static-option

`static-option` is a [rust](https://rust-lang.org) library that provides versions of `Option` and `Result` that track their state at compile time using const generic boolean parameters.

## Features
* Statically tracks if a `StaticOption` contains a value or a `StaticResult` is `ok` or `err` at compile time.
* Direct access to the contents of `StaticOption` and `StaticResult` if the type parameters guarantee that there is a value inside.
* Optimal memory layout using `unsafe` internally and exposing a safe interface so that you don't have to write `unsafe` yourself
  * `StaticOption` uses `MaybeUninit`
  * `StaticResult` uses a `union`
* MSRV (minimum supported rust version) of 1.51, where `min_const_generics` where introduced.
* All standard library functions and traits of `Option` and `Result` that can be reimplement for `StaticOption` and `StaticResult` are reimplemented. (if one is missing, open a GitHub issue about it)
* `#![no_std]`
* Conversion back to the standard `Option` and `Result` types.

## Caveats
* Some methods from the standard library cannot be implemented on `StaticOption` and `StaticResult`
  * Methods that mutably change the content from `some` -> `none`, `none` -> `some` or `ok` -> `error`, `error` -> `ok` respectively.
  * Methods that require boolean logic between to const generic boolean type parameters, like `Option::xor` for example.
* `StaticOption` and `StaticResult` do not implement `Drop`, this is because they have no way to track if the content's have been dropped yet.
  * If you aren't using any method taking owned `self` as parameter, you need to make sure to call `.drop()` manually.
  * For that reason, bot `StaticOption` and `StaticResult` emit a warning if they aren't used, thanks to the `#[must_use]` attribute.

## Example: Statically checked builder pattern

Example on how `StaticOption` can be used, implementing compile time checked builder pattern.

**NOTE:** If you actually want to use builders like that, I recommend the excellent [typed-builder](https://github.com/idanarye/rust-typed-builder) instead

```rust
use static_option::StaticOption;

#[derive(Debug, PartialEq)]
struct Point {
	x: f64,
	y: f64,
}

impl Point {
	// Starts with both const generic type parameters as `false`
	// (in other words: Neither `x` nor `y` having been set)
	pub fn build() -> Builder<false, false> {
		Builder {
			x: Default::default(),
			y: Default::default(),
		}
	}
}

// The X and Y const generic type parameters track at compile time which values have already been provided to the builder.
// The actual values are stored inside of `StaticOption`.
#[must_use = "Finish building with `.build()` if you don't use the `Builder` anymore."]
struct Builder<const X: bool, const Y: bool> {
	x: StaticOption<f64, X>,
	y: StaticOption<f64, Y>,
}

// Setting `x` is only possible on builders where the `X` type parameter is `false` and it will set it to `true`.
// (in other words: Where `x` hasn't been set yet)
impl<const Y: bool> Builder<false, Y> {
	pub fn x(self, x: f64) -> Builder<true, Y> {
 		Builder {
			x: x.into(),
			y: self.y,
		}
	}
}

// Setting `y` is only possible on builders where the `Y` type parameter is `false` and it will set it to `true`.
// (in other words: Where `y` hasn't been set yet)
impl<const X: bool> Builder<X, false> {
	pub fn y(self, y: f64) -> Builder<X, true> {
 		Builder {
			x: self.x,
			y: y.into(),
		}
	}
}

// The `build` method is only available when both `X` and `Y` type parameters are `true`
// (in other words: When both `x` and `y` have been set)
impl Builder<true, true> {
	pub fn build(self) -> Point {
		Point {
			x: self.x.into_inner(),
			y: self.y.into_inner(),
		}
	}
}

let point = Point::build().x(1.0).y(2.0).build();
let point2 = Point::build().y(2.0).x(1.0).build();
let expected = Point { x: 1.0, y: 2.0 };

assert_eq!(expected, point);
assert_eq!(point, point2);
```
