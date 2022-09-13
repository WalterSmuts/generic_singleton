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
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Mul};

// The expensive function we're trying to cache using a singleton map
fn multiply<T: Mul<Output = T>>(a: T, b: T) -> T {
    a * b
}

// Private struct to use in the `get_or_init` macro
struct MyPrivateHashMap<T: Mul<Output = T>>(HashMap<(T, T), T>);

impl<T: Mul<Output = T>> Deref for MyPrivateHashMap<T> {
    type Target = HashMap<(T, T), T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Mul<Output = T>> DerefMut for MyPrivateHashMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn multiply_with_cache<T: Mul<Output = T>>(a: T, b: T) -> T
where
    T: std::cmp::Eq,
    T: Copy,
    T: 'static,
    (T, T): std::hash::Hash,
{
    // This is a generic singleton map!!!
    let map = generic_singleton::get_or_init!(|| RefCell::new(MyPrivateHashMap(HashMap::new())));
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
    // This call will create the AnyMap and the MyPrivateHashMap<i32> and do the multiplication
    multiply_with_cache::<u32>(10, 10);
    // This call will only retrieve the value of the multiplication from MyPrivateHashMap<i32>
    multiply_with_cache::<u32>(10, 10);

    // This call will create a second MyPrivateHashMap< and do the multiplication
    multiply_with_cache::<i32>(-1, -10);
    // This call will only retrieve the value of the multiplication from MyPrivateHashMap
    multiply_with_cache::<i32>(-1, -10);
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
