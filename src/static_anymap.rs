use anymap3::AnyMap;
use parking_lot::RwLock;
use std::ptr::NonNull;

/// A wrapper around `NonNull<T>` that implements `Any` so it can be stored in an `AnyMap`.
///
/// We use `NonNull<T>` instead of `Box<T>` to avoid `Box`'s aliasing assertions.
/// `Box<T>` is considered equivalent to `&mut T` for aliasing purposes, so storing
/// `Box<T>` in the map and later creating `&mut AnyMap` (via `entry()`) would alias
/// any previously handed-out `&'static T` references. `NonNull` is a raw pointer and
/// does not participate in borrow tracking, breaking the aliasing chain.
struct Leaked<T>(NonNull<T>);

// SAFETY: The `T: Sync` bound on all public accessors ensures the pointee is safe to
// share across threads. We never hand out `&mut T`.
unsafe impl<T: Sync> Send for Leaked<T> {}
// SAFETY: Same as above — T: Sync means &T is safe to share.
unsafe impl<T: Sync> Sync for Leaked<T> {}

/// The point of this struct is to wrap the AnyMap in a concurrent, static version that will only
/// insert items that have 'static lifetimes. This is achieved by leaking allocations via
/// `NonNull<T>` and never removing items. This module only exposes the StaticAnyMap struct and
/// its get_or_init method. Using these should be perfectly safe.
#[derive(Default)]
pub struct StaticAnyMap {
    inner: RwLock<AnyMap>,
}

// SAFETY:
// From the rustonomicon:
// https://doc.rust-lang.org/nomicon/send-and-sync.html
// "A type is Sync if it is safe to share between threads (T is Sync if and only if &T is Send)."
//
// All accessors are only implemented for `T: Sync`, so we can say &T ('static implied) is Send.
//
// Since we never return `&mut T` we can safely store `!Send` data as long as `StaticAnyMap`
// remains `!Send` (which it currently is, though it would be nice to have a test for that).
// Since any accessor implicitly requires `T: 'static` we can assume there's no dangling data.
//
// `AnyMap` itself is `!Send + !Sync` because it can hold arbitrary types, but we guard
// all access through a `RwLock`, so concurrent access is safe. `Send` is needed for
// `OnceLock<StaticAnyMap>` in a static context. This is safe because we never expose
// `&mut AnyMap` outside the lock, and all inserted values are `Sync + 'static`.
unsafe impl Sync for StaticAnyMap {}
// SAFETY: See above.
unsafe impl Send for StaticAnyMap {}

impl StaticAnyMap {
    #[must_use]
    pub fn get_or_init<T: Sync + 'static>(&'static self, init: impl FnOnce() -> T) -> &'static T {
        // Fast path: read lock only
        {
            let map = self.inner.read();
            if let Some(leaked) = map.get::<Leaked<T>>() {
                // SAFETY: The pointer was created from `Box::into_raw` and is never freed,
                // so it is valid for 'static. We only hand out shared references and T: Sync.
                return unsafe { leaked.0.as_ref() };
            }
        }

        // Slow path: write lock, use or_insert_with to guarantee init runs at most once
        let mut map = self.inner.write();
        let leaked = map.entry().or_insert_with(|| {
            // SAFETY: `Box::into_raw` returns a valid, aligned, non-null pointer.
            let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(init()))) };
            Leaked(ptr)
        });
        // SAFETY: Same as fast path.
        unsafe { leaked.0.as_ref() }
    }
}

// Compile tests

// Note: compile_fail tests need to be in a public module that exists even when `cfg(not(test))`
// otherwise the compiler won't execute them.

#[allow(unused)]
fn check_send()
where
    StaticAnyMap: Send,
{
}

#[allow(unused)]
fn check_sync()
where
    StaticAnyMap: Sync,
{
}

/// ```compile_fail
/// use generic_singleton::static_anymap::StaticAnyMap;
///
/// fn check_t_needs_sync<T: Default + 'static>(map: &'static StaticAnyMap) {
///     map.get_or_init::<T>(T::default);
/// }
/// ```
const _: () = ();

/// ```compile_fail
/// use generic_singleton::static_anymap::StaticAnyMap;
///
/// fn check_t_needs_static<T: Default + Sync>(map: &'static StaticAnyMap) {
///     map.get_or_init::<T>(T::default);
/// }
/// ```
const _: () = ();

#[allow(unused)]
fn check_t_needs_sync_not_send<T: Default + Sync + 'static>(map: &'static StaticAnyMap) {
    map.get_or_init::<T>(T::default);
}
