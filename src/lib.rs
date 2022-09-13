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
/// fn multiply_with_cache<T: Mul<Output = T>>(a: T, b: T) -> T
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
