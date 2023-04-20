use anymap::AnyMap;
use std::cell::{RefCell, UnsafeCell};

/// The point of this struct is to wrap the AnyMap in a thread local, version that will only insert
/// items that have 'static lifetimes. This is acieved by wrapping all items in `Box<T>` and never
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
    pub fn get_or_init_with<T: 'static>(
        &self,
        init: impl FnOnce() -> T,
        with: impl FnOnce(&mut T),
    ) {
        let optional_t: &UnsafeCell<Option<RefCell<T>>> = {
            // SAFETY:
            // The pointer returned by `self.inner.get()` is guarantee to be valid, properly aligned
            // and point to a valid `T` because it comes from `self.inner`, which must be valid.
            //
            // The only tricky requisite is guaranteeing that there aren't aliasing references. This is
            // true because `ThreadLocalStaticAnymap` is `!Sync`, so this must be the only thread
            // with a reference to it, and we never call `init` or `with` when we're holding a reference
            // to `map` because its scope is limited to this block.
            //
            // Note that while `optional_t` borrows from `map`, it does from a value inside a
            // `Pin<Box<_>>`, which will remain valid even if others borrow from `map`.
            let map = unsafe { &mut *self.inner.get() };
            map.entry()
                .or_insert_with(|| Box::pin(UnsafeCell::new(None)))
        };

        // TODO: Replace the `Option` with `std::cell::Once`, this will allow to avoid any
        // `unsafe` needed to access `optional_t`.

        // SAFETY:
        // Nothing is borrowing `optional_t` at the moment, nor we run `init` or `with` while
        // we're holding on this borrow.
        if unsafe { (*optional_t.get()).is_none() } {
            let value = init();

            // If now `optional_t` is `some` it means `init` reentrantly accessed `self`
            // and initialized the value for `T`.
            //
            // SAFETY:
            // Nothing is borrowing `optional_t` at the moment, nor we run `init` or `with` while
            // we're holding on this borrow.
            assert!(unsafe { (*optional_t.get()).is_none() }, "reentrant init");

            // SAFETY:
            // Nothing is borrowing `optional_t` at the moment, nor we run `init` or `with` while
            // we're holding on this borrow.
            unsafe { *optional_t.get() = Some(RefCell::new(value)) };
        }

        // SAFETY:
        // Nothing is borrowing `optional_t` at the moment, and running `with` later is safe because
        // we initialized `optional_t` with `Some`, so even if `with` reentrantly accesses `self`
        // it won't be able to write to `optional_t` or even create a mutable reference, so this
        // reference will remain valid.
        let optional_t_ref = unsafe { &*optional_t.get() };

        // SAFETY:
        // `optional_t_ref` is guaranteed to be `Some` because if it were `None` then it would
        // have been initialized at the end of the previous `if`.
        let t_ref = unsafe { optional_t_ref.as_ref().unwrap_unchecked() };

        // Note: The `RefCell` here is needed because `with` could reentratly access `self`.
        // This is the same reason as for why `thread_local!` gives out only an `&T` in its `with`.
        with(&mut *t_ref.borrow_mut())
    }
}
