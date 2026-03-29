use anymap3::AnyMap;
use parking_lot::RwLock;

/// The point of this struct is to wrap the AnyMap in a concurrent, static version that will only
/// insert items that have 'static lifetimes. This is achieved by leaking allocations via
/// `Box::leak` and storing the resulting `&'static T` directly in the map. This module only
/// exposes the StaticAnyMap struct and its get_or_init method. Using these should be perfectly
/// safe.
#[derive(Default)]
pub struct StaticAnyMap {
    inner: RwLock<AnyMap>,
}

// SAFETY:
// `AnyMap` itself is `!Send + !Sync` because it can hold arbitrary types, but we guard
// all access through a `RwLock`, so concurrent access is safe. All stored values are
// `&'static T` where `T: Sync`, so they are safe to share across threads. `Send` is
// needed for `OnceLock<StaticAnyMap>` in a static context.
unsafe impl Sync for StaticAnyMap {}
// SAFETY: See above.
unsafe impl Send for StaticAnyMap {}

impl StaticAnyMap {
    #[must_use]
    pub fn get_or_init<T: Sync + 'static>(&'static self, init: impl FnOnce() -> T) -> &'static T {
        // Fast path: read lock only
        {
            let map = self.inner.read();
            if let Some(r) = map.get::<&'static T>() {
                return r;
            }
        }

        // Slow path: write lock, use or_insert_with to guarantee init runs at most once
        let mut map = self.inner.write();
        map.entry().or_insert_with(|| {
            let leaked: &'static T = Box::leak(Box::new(init()));
            leaked
        })
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
