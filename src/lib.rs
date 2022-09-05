#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![doc = include_str!("../README.md")]

use anymap::AnyMap;
use std::cell::UnsafeCell;

/// Get a static reference to a generic singleton or initialize it if it doesn't exist.
///
/// You can even call [get_or_init] inside the init function, although, if you're initializing the
/// same type, the outer call will persist. If this example confuses you, don't worry, it's a
/// contrived non-sensical example that resulted from testing the safety of this library.
pub fn get_or_init<T: 'static>(init: fn() -> T) -> &'static T {
    thread_local! {
        static REF_CELL_MAP: UnsafeCell<AnyMap> = UnsafeCell::new(AnyMap::new());
    };
    REF_CELL_MAP.with(|map_cell| {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works() {
        let a = get_or_init(|| 0);
        let b = get_or_init(|| 1);
        assert_eq!(a, b);
    }

    #[test]
    fn recursive_call_to_get_or_init_does_not_panic() {
        get_or_init(|| get_or_init(|| 0));
    }
}
