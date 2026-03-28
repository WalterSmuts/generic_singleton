use anymap3::AnyMap;
use std::cell::OnceCell;
use std::cell::UnsafeCell;
use std::ptr::NonNull;

/// A wrapper around `NonNull<T>` that implements `Any` so it can be stored in an `AnyMap`.
///
/// We use `NonNull<T>` instead of `Box<T>` to avoid `Box`'s aliasing assertions.
/// See `static_anymap::Leaked` for the full rationale.
struct Leaked<T>(NonNull<T>);

impl<T> Drop for Leaked<T> {
    fn drop(&mut self) {
        // SAFETY: The pointer was created from `Box::into_raw`.
        unsafe { drop(Box::from_raw(self.0.as_ptr())) };
    }
}

/// The point of this struct is to wrap the AnyMap in a thread local version that will only insert
/// items that have 'static lifetimes. This is achieved by leaking allocations via `NonNull<T>`
/// and never removing items. This module only exposes the ThreadLocalStaticAnymap struct and its
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
            // `map` because its scope is limited to this block. The returned `NonNull` points
            // to a heap allocation that is never freed, so it remains valid. Using `NonNull`
            // instead of `Box` avoids aliasing issues: reentrant calls that create a new
            // `&mut *self.inner.get()` do not invalidate pointers derived from `NonNull`.
            let map = unsafe { &mut *self.inner.get() };
            let leaked = map.entry().or_insert_with(|| {
                // SAFETY: `Box::into_raw` returns a valid, aligned, non-null pointer.
                let ptr =
                    unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(OnceCell::new()))) };
                Leaked(ptr)
            });
            // SAFETY: The pointer was created from `Box::into_raw` and is never freed.
            // No `&mut OnceCell<T>` is ever created from this pointer.
            unsafe { leaked.0.as_ref() }
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
