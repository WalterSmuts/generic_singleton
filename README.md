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
singleton in thread local storage. Subsequent calls to `get_or_init` will
retrieve the singleton from the map.

### Example
```rust
use std::{cell::RefCell, ops::AddAssign};

use num_traits::{One, Zero};

fn generic_call_counter<T: Zero + One + Copy + AddAssign + 'static>() -> T {
    let mut count = generic_singleton::get_or_init!(|| RefCell::new(T::zero())).borrow_mut();
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

### Current limitations:
- The map is guarded by an RwLock for thread-safety. This is unnecessary in
  single-threaded situations.

[static generic items]: https://doc.rust-lang.org/reference/items/static-items.html#statics--generics
[anymap]: https://docs.rs/anymap/latest/anymap/
[new type pattern]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
