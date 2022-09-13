#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![doc = include_str!("../README.md")]
use anymap::AnyMap;
use std::cell::UnsafeCell;
pub extern crate anymap;

/// Get a static reference to a generic singleton or initialize it if it doesn't exist.
///
/// ### Example
/// ```rust
/// use std::sync::Mutex;
///
/// fn generic_exponential_counter<T: Copy + std::ops::Add<Output = T> + 'static>(first_value: T) -> T {
///     {
///         let mut a = generic_singleton::get_or_init!(|| Mutex::new(first_value)).lock().unwrap();
///         let b = *a;
///         *a = *a + b;
///         *a
///     }
/// }
///
/// fn main() {
///     // Works with i32
///     assert_eq!(generic_exponential_counter(2), 4);
///     assert_eq!(generic_exponential_counter(2), 8);
///     assert_eq!(generic_exponential_counter(2), 16);
///
///     // Works with f64
///     assert_eq!(generic_exponential_counter(2.0), 4.0);
///     assert_eq!(generic_exponential_counter(2.0), 8.0);
///     assert_eq!(generic_exponential_counter(2.0), 16.0);
/// }
/// ```
#[macro_export]
macro_rules! get_or_init {
    ($init:expr) => {{
        use std::cell::UnsafeCell;
        use $crate::anymap::AnyMap;
        use $crate::get_from_map;

        thread_local! {
            static UNSAFE_CELL_MAP: UnsafeCell<AnyMap> = UnsafeCell::new(AnyMap::new());
        };
        UNSAFE_CELL_MAP.with(|map_cell| get_from_map(map_cell, $init))
    }};
}

/// Used by the [`get_or_init`] macro to access the generic item from the map. Do not use
/// directly. Use the [`get_or_init`] macro instead.
pub fn get_from_map<T>(map_cell: &UnsafeCell<AnyMap>, init: impl FnOnce() -> T) -> &'static T {
    // Curly brackets are important here to drop the borrow after checking
    let contains = {
        // SAFETY:
        // This was tested using a RefCell in the following commit:
        // d23f9f28ed6b9d1b61cc82e9e2cff17c7f4575c1
        let map = unsafe { map_cell.get().as_ref().unwrap() };
        map.contains::<T>()
    };
    if !contains {
        // Important to calculate this value BEFORE we borrow the map
        let val = init();
        // SAFETY:
        // This was tested using a RefCell in the following commit:
        // d23f9f28ed6b9d1b61cc82e9e2cff17c7f4575c1
        let map = unsafe { map_cell.get().as_mut().unwrap_unchecked() };
        map.insert(val);
        // Drop the borrow
    }
    // SAFETY:
    // This was tested using a RefCell in the following commit:
    // d23f9f28ed6b9d1b61cc82e9e2cff17c7f4575c1
    let map = unsafe { map_cell.get().as_ref().unwrap() };
    // SAFETY:
    // The function will only return None if the item is not present. Since we always add the
    // item if it's not present two lines above and never remove items, we can be sure that
    // this function will always return `Some`.
    let t_ref = unsafe { map.get::<T>().unwrap_unchecked() };
    let ptr = t_ref as *const T;
    // SAFETY:
    // Check: The pointer must be properly aligned.
    // Proof: The pointer is obtained from a valid reference so it must be aligned.
    //
    // Check: It must be “dereferenceable” in the sense defined in the module documentation.
    // Proof: The pointer is obtained from a valid reference it musts be dereferenceable.
    //
    // Check: The pointer must point to an initialized instance of T.
    // Proof: The AnyMap crate provides this guarantee.
    //
    // Check: You must enforce Rust’s aliasing rules, since the returned lifetime 'a is
    //        arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
    //        In particular, while this reference exists, the memory the pointer points to
    //        must not get mutated (except inside UnsafeCell).
    // Proof: We return a shared reference and therefore cannot be mutated unless T is
    //        guarded with the normal rust memory protection constructs using UnsafeCell.
    //        The data could be dropped if we ever removed it from this map however. Care
    //        must be taken to never introduce any logic that would remove T from the map.
    let optional_ref = unsafe { ptr.as_ref() };
    // SAFETY:
    // This requires the pointer not to be null. We obtained the pointer one line above
    // from a valid reference, therefore this is considered safe to do.
    unsafe { optional_ref.unwrap_unchecked() }
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
}
