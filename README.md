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
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Mul};

// The expensive function we're trying to cache using a singleton map, however,
// we want the user of the function to determine the type of the elements being
// multiplied.
fn multiply<T: Mul<Output = T>>(a: T, b: T) -> T {
    a * b
}

fn multiply_with_cache<T: Mul<Output = T>>(a: T, b: T) -> T
where
    T: std::cmp::Eq,
    T: Copy,
    T: 'static,
    (T, T): std::hash::Hash,
{
    // This is a generic singleton map!!!
    let map = generic_singleton::get_or_init!(|| RefCell::new(HashMap::new()));
    let key = (a, b);
    if map.borrow().contains_key(&key) {
        *map.borrow().get(&key).unwrap()
    } else {
        let result = multiply(a, b);
        map.borrow_mut().insert(key, result);
        result
    }
}

fn main() {
    // This call will create the AnyMap and the HashMap<i32> and do the multiplication
    multiply_with_cache::<u32>(10, 10);
    // This call will only retrieve the value of the multiplication from HashMap<i32>
    multiply_with_cache::<u32>(10, 10);

    // This call will create a second HashMap< and do the multiplication
    multiply_with_cache::<i32>(-1, -10);
    // This call will only retrieve the value of the multiplication from HashMap
    multiply_with_cache::<i32>(-1, -10);
}
```

### Current limitations:
- Each thread has a different map. I may add `get_or_init_local` and `get_or_init_global` if I
can figure out how to implement concurrent access and the required trait bound to make it safe.

[static generic items]: https://doc.rust-lang.org/reference/items/static-items.html#statics--generics
[anymap]: https://docs.rs/anymap/latest/anymap/
[new type pattern]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
