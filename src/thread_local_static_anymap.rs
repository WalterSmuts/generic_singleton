use anymap::AnyMap;
use std::cell::UnsafeCell;

/// The point of this struct is to wrap the AnyMap in a thread local, version that will only insert
/// items that have 'static lifetimes. This is acieved by wrapping all items in Box<T> and never
/// removing items. This module only exposes the ThreadLocalStaticAnymap struct and it's
/// get_or_init_with method. Using these should be perfectly safe.
#[derive(Default)]
pub struct ThreadLocalStaticAnymap {
    inner: UnsafeCell<AnyMap>,
}

// SAFETY:
// It's assumed that this Type will only be initialized using a `thread_local!` macro. The rest of
// the SAFETY comments assume this type is used from thread local storage and therefore issues
// related to concurrent execution is not considered.
impl ThreadLocalStaticAnymap {
    /// Safety:
    /// Users need to ensure this is only called from a ThreadLocalStaticAnymap in thread local
    /// storge and that the `with` closure doesn't contain references to the same
    /// ThreadLocalStaticAnymap.
    pub unsafe fn get_or_init_with<T: 'static>(
        &self,
        init: impl FnOnce() -> T,
        mut with: impl FnMut(&mut T),
    ) {
        let map_pointer = self.inner.get();
        // SAFETY:
        // Check:
        // * The pointer must be properly aligned.
        // * It must be "dereferenceable" in the sense defined in [the module documentation].
        // * The pointer must point to an initialized instance of `T`.
        // Proof:
        // * The pointer was allocated via normal rust methods in safe code and obtained from an
        // `UnsafeCell` one line above. The pointer cannot be unaligned and has to be
        // dereferenceable.
        //
        // Check:
        // * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
        //   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
        //   In particular, while this reference exists, the memory the pointer points to must
        //   not get accessed (read or written) through any other pointer.
        // Proof:
        // * This can be broken by referencing the same ThreadLocalStaticAnymap in the `with`
        // closure. That's why this function is still marked `unsafe`. Any user needs to ensure
        // there's no way that the same ThreadLocalStaticAnymap can be used in the `with` closure.
        let optional_map = unsafe { map_pointer.as_mut() };

        // SAFETY:
        // The map is always created normally, therefore this pointer cannot be null.
        let map = unsafe { optional_map.unwrap_unchecked() };
        if !map.contains::<UnsafeCell<Box<T>>>() {
            map.insert(UnsafeCell::new(Box::new(init())));
        };
        let optional_unsafe_t_ref = map.get::<UnsafeCell<Box<T>>>();
        // SAFETY:
        // We always insert into the map if it doesn't exist 3 lines above so this must be a Some
        // variant.
        let unsafe_t_ref = unsafe { optional_unsafe_t_ref.unwrap_unchecked() };

        let t_pointer = unsafe_t_ref.get();
        // SAFETY:
        // Check:
        // * The pointer must be properly aligned.
        // * It must be "dereferenceable" in the sense defined in [the module documentation].
        // Proof:
        // * The pointer was allocated via normal rust methods in safe code and obtained from an
        // UnsafeCell one line above. The pointer cannot be unaligned and has to be
        // dereferenceable.
        //
        // Check:
        // * The pointer must point to an initialized instance of `T`.
        // Proof:
        // The AnyMap crate guarantees this for us.
        //
        // Check:
        // * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
        //   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
        //   In particular, while this reference exists, the memory the pointer points to must
        //   not get accessed (read or written) through any other pointer.
        // Proof:
        // * This can be broken by referencing the same ThreadLocalStaticAnymap in the `with`
        // closure. That's why this function is still marked `unsafe`. Any user needs to ensure
        // there's no way that the same ThreadLocalStaticAnymap can be used in the `with` closure.
        let optional_t_ref = unsafe { t_pointer.as_mut() };

        // SAFETY:
        // The reference we get back from AnyMap is already proved to be valid, therefore this
        // cannot be null either.
        let t_ref = unsafe { optional_t_ref.unwrap_unchecked() };
        with(t_ref);
    }
}
