# generic\_singleton
Rust does NOT monomorphize it's [static generic items]. This is dissappointing when you have a
generic function that contains a static variable depending on that generic. You'll get the
following error:
```text
error[E0401]: can't use generic parameters from outer function
```

That's pretty frustrating when you want to write a singleton pattern that rely's on a generic
parameter. That's where this crate saves the day!

`generic_singleton` uses [anymap] behind the scenes to store a map of each generic type. The first
time you call `get_or_init` we initialize the singleton in thread local storage. Subsequent
calls to `get_or_init` will retrieve the singleton from the map.

__DO NOT USE WITH A TYPE YOUR CRATE DOES NOT PRIVATELY OWN!!!__ The map is
shared across crates, so if you use a type that is publicly available, then
another crate will be able to mutate your singleton, breaking whichever rules
you've got in place and vice-versa. Use the [new type pattern] if you need to
use a public struct in the map.

### Example
```rust
use std::sync::Mutex;

fn generic_function<T: Copy + std::ops::Add<Output = T> + 'static>(initializer: fn() -> Mutex<T>) -> T {
    {
        let mut a = generic_singleton::get_or_init(initializer).lock().unwrap();
        let b = *a;
        *a = *a + b;
        *a
    }
}

fn main() {
    assert_eq!(generic_function(||Mutex::new(2)), 4);
    assert_eq!(generic_function(||Mutex::new(2)), 8);
    assert_eq!(generic_function(||Mutex::new(2)), 16);

    assert_eq!(generic_function(||Mutex::new(2.0)), 4.0);
    assert_eq!(generic_function(||Mutex::new(2.0)), 8.0);
    assert_eq!(generic_function(||Mutex::new(2.0)), 16.0);
}
```

### Current limitations:
- Each thread has a different map. I may add `get_or_init_local` and `get_or_init_global` if I
can figure out how to implement concurrent access and the required trait bound to make it safe.
- There is only one map(per thread). Multiple uses of `get_or_init` on the same type will
return the __SAME__ singleton. __Even across crate boundaries!__ I might be able to limit
it by providing a macro that salts the key in `AnyMap` with a context about which function it's
being called from.

[static generic items]: https://doc.rust-lang.org/reference/items/static-items.html#statics--generics
[anymap]: https://docs.rs/anymap/latest/anymap/
[new type pattern]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
