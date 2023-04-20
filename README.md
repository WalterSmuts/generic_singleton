# generic\_singleton
Rust does NOT monomorphize it's [static generic items]. This means you cannot
use a generic static item in a generic function. You'll get the following
error:
```text
error[E0401]: can't use generic parameters from outer function
```

That's pretty frustrating when you want to write a singleton pattern that rely's on a generic
parameter. This crate allows for this pattern with minimal runtime overhead.

`generic_singleton` uses [anymap] behind the scenes to store a map of each
generic type. The first time you hit the `get_or_init` macro we initialize the
singleton. Subsequent calls to `get_or_init` will retrieve the singleton from
the map.

### Example
```rust
use std::{ops::AddAssign, sync::RwLock};

use num_traits::{One, Zero};

fn generic_call_counter<T: Zero + One + Copy + AddAssign + Send + Sync + 'static>() -> T {
    let mut count = generic_singleton::get_or_init!(|| RwLock::new(T::zero())).write().unwrap();
    *count += T::one();
    *count
}

fn main() {
    // Works with usize
    assert_eq!(generic_call_counter::<usize>(), 1);
    assert_eq!(generic_call_counter::<usize>(), 2);
    assert_eq!(generic_call_counter::<usize>(), 3);

    // Works with i32
    assert_eq!(generic_call_counter::<i32>(), 1);
    assert_eq!(generic_call_counter::<i32>(), 2);
    assert_eq!(generic_call_counter::<i32>(), 3);

    // Works with f32
    assert_eq!(generic_call_counter::<f32>(), 1.0);
    assert_eq!(generic_call_counter::<f32>(), 2.0);
    assert_eq!(generic_call_counter::<f32>(), 3.0);
}
```

### Thread Local variant

The example shown above has a drawback of requiring an `RwLock` to ensure
synchronisation around the inner [AnyMap]. In single-threaded situations we can
remove this lock and provide mutable references directly using the
`get_or_init_thread_local!` macro. This comes at the cost of ergonomics,
requiring you to express your logic in a closure rather than simply returning a
reference.

[static generic items]: https://doc.rust-lang.org/reference/items/static-items.html#statics--generics
[anymap]: https://docs.rs/anymap/latest/anymap/
[AnyMap]: https://docs.rs/anymap/latest/anymap/type.AnyMap.html
[new type pattern]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
