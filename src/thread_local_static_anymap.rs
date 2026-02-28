use anymap3::AnyMap;
use std::cell::OnceCell;
use std::cell::UnsafeCell;

/// The point of this struct is to wrap the AnyMap in a thread local version that will only insert
/// items that have 'static lifetimes. This is achieved by wrapping all items in `Box<T>` and never
/// removing items. This module only exposes the ThreadLocalStaticAnymap struct and its
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
    /// storage and that the `with` closure doesn't contain references to the same
    /// ThreadLocalStaticAnymap.
    pub fn get_or_init_with<T: 'static>(&self, init: impl FnOnce() -> T, with: impl FnOnce(&T)) {
        let cell: &OnceCell<T> = {
            // SAFETY:
            // `ThreadLocalStaticAnymap` is `!Sync`, so this must be the only thread with a
            // reference to it. We never call `init` or `with` while holding a reference to
            // `map` because its scope is limited to this block. The returned `OnceCell` lives
            // inside a `Box<_>` so it remains valid even if others borrow from `map`.
            let map = unsafe { &mut *self.inner.get() };
            map.entry().or_insert_with(|| Box::new(OnceCell::new()))
        };

        let t_ref = cell.get_or_init(init);
        with(t_ref)
    }
}

// Compile tests

// Note: compile_fail tests need to be in a public module that exists even when `cfg(not(test))`
// otherwise the compiler won't execute them.

/// ```compile_fail
/// use generic_singleton::thread_local_static_anymap::ThreadLocalStaticAnymap;
///
/// fn check_not_send() where ThreadLocalStaticAnymap: Send {}
/// ```
const _: () = ();

/// ```compile_fail
/// use generic_singleton::thread_local_static_anymap::ThreadLocalStaticAnymap;
///
/// fn check_not_sync() where ThreadLocalStaticAnymap: Sync {}
/// ```
const _: () = ();

/// ```compile_fail
/// use generic_singleton::thread_local_static_anymap::ThreadLocalStaticAnymap;
///
/// fn check_t_needs_static<T: Default>(map: &'static ThreadLocalStaticAnymap) {
///     map.get_or_init_with::<T>(T::default, |_| ());
/// }
/// ```
const _: () = ();

#[allow(unused)]
fn check_t_needs_not_sync_not_send<T: Default + 'static>(map: &'static ThreadLocalStaticAnymap) {
    map.get_or_init_with::<T>(T::default, |_| ());
}
