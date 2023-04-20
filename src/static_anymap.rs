use std::pin::Pin;

use anymap::AnyMap;
use parking_lot::RwLock;

/// The point of this struct is to wrap the AnyMap in a concurrent, static version that will only
/// insert items that have 'static lifetimes. This is acieved by wrapping all items in
/// `Pin<Box<T>>` and never removing items. This module only exposes the StaticAnyMap struct and
/// it's get_or_init method. Using these should be perfectly safe.
#[derive(Default)]
pub struct StaticAnyMap {
    inner: RwLock<AnyMap>,
}

// SAFETY:
// From the rustonomicon:
// https://doc.rust-lang.org/nomicon/send-and-sync.html
// "A type is Sync if it is safe to share between threads (T is Sync if and only if &T is Send)."
// All accessors are only implemented for 'static references, so I believe we can say &T ('static
// implied) is Send. And since any mutation is guarded by a RwLock, the type should be sync too.
//
// I'm a bit uncertain about the above. I've managed to limit access to the inner AnyMap via
// 'static references, but there's nothing stopping a user from creating a non-'static
// StaticAnyMap. The impl below allows to send it between threads, even if the type is not
// 'static. This instance would be unusable but perhaps it can still trigger UB. For example in
// the following podcast at ~14:10, Andrew Kelly mentions the existence of a dangling pointer
// might cause undefined behaviour:
// https://rustacean-station.org/episode/andrew-kelley/
// I'm concerned that, similar to the dangling pointer example, the existence of a struct that is
// shared across thread boundaries which shouldn't already triggers UB regardless of whether it's
// actually accessed.
unsafe impl Sync for StaticAnyMap {}

impl StaticAnyMap {
    fn get<T: Sync + 'static>(&'static self) -> Option<&'static T> {
        let read_only_guard = self.inner.read();
        let val = read_only_guard.get::<Pin<Box<T>>>()?;
        // SAFETY:
        // Since we only insert values into the map and we wrap the values in Pin<Box<T>> and the
        // map itelf has a static lifetime, we can be sure that the data being pointed to has
        // static lifetime.
        Some(unsafe { convert_to_static_ref(val) })
    }

    fn init_and_return<T: Sync + 'static>(&'static self, init: impl FnOnce() -> T) -> &'static T {
        let mut writeable_map = self.inner.write();

        let val = writeable_map.entry().or_insert_with(|| Box::pin(init()));

        // SAFETY:
        // Since we only insert values into the map and we wrap the values in Pin<Box<T>> and the
        // map itelf has a static lifetime, we can be sure that the data being pointed to has
        // static lifetime.
        unsafe { convert_to_static_ref(val) }
    }

    pub fn get_or_init<T: Sync + 'static>(&'static self, init: impl FnOnce() -> T) -> &'static T {
        if let Some(val) = self.get() {
            val
        } else {
            self.init_and_return(init)
        }
    }
}

// # Safety
// The reference MUST point to data that has a static lifetime and otherwise be a valid reference.
unsafe fn convert_to_static_ref<T>(pin: &Pin<Box<T>>) -> &'static T {
    let t_ref = &*pin.as_ref();
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
