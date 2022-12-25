#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![doc = include_str!("../README.md")]
pub extern crate anymap;

#[doc(hidden)]
pub mod static_anymap;
pub extern crate lazy_static;

/// Get a static reference to a generic singleton or initialize it if it doesn't exist.
///
/// ### Example
/// ```rust
/// use std::cell::RefCell;
/// use std::collections::HashMap;
/// use std::ops::{Deref, DerefMut, Mul};
///
/// // The expensive function we're trying to cache using a singleton map, however,
/// // we want the user of the function to determine the type of the elements being
/// // multiplied.
/// fn multiply<T: Mul<Output = T>>(a: T, b: T) -> T {
///     a * b
/// }
///
/// fn multiply_with_cache<T: Mul<Output = T> + Send>(a: T, b: T) -> T
/// where
///     T: std::cmp::Eq,
///     T: Copy,
///     T: 'static,
///     (T, T): std::hash::Hash,
/// {
///     // This is a generic singleton map!!!
///     let map = generic_singleton::get_or_init!(|| RefCell::new(HashMap::new()));
///     let key = (a, b);
///     if map.borrow().contains_key(&key) {
///         *map.borrow().get(&key).unwrap()
///     } else {
///         let result = multiply(a, b);
///         map.borrow_mut().insert(key, result);
///         result
///     }
/// }
///
/// fn main() {
///     // This call will create the AnyMap and the HashMap<i32> and do the multiplication
///     multiply_with_cache::<u32>(10, 10);
///     // This call will only retrieve the value of the multiplication from HashMap<i32>
///     multiply_with_cache::<u32>(10, 10);
///
///     // This call will create a second HashMap< and do the multiplication
///     multiply_with_cache::<i32>(-1, -10);
///     // This call will only retrieve the value of the multiplication from HashMap
///     multiply_with_cache::<i32>(-1, -10);
/// }
/// ```
#[macro_export]
macro_rules! get_or_init {
    ($init:expr) => {{
        use $crate::lazy_static::lazy_static;
        use $crate::static_anymap::StaticAnyMap;

        lazy_static! {
            static ref STATIC_ANY_MAP: StaticAnyMap = StaticAnyMap::default();
        }
        STATIC_ANY_MAP.get_or_init($init)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testing_function() -> &'static i32 {
        get_or_init!(|| 0)
    }

    #[test]
    fn works() {
        let a = testing_function();
        let b = testing_function();
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn recursive_call_to_get_or_init_does_not_panic() {
        get_or_init!(|| get_or_init!(|| 0));
    }

    #[test]
    fn initializing_in_different_thread_works() {
        let v = std::thread::spawn(|| get_or_init!(|| String::from("foo")))
            .join()
            .unwrap();

        println!("{v}"); // Check for use-after-free, to be sure bugs are caught, run with miri
    }
}
