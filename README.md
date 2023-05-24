# `consume_on_drop`

**A zero-cost abstraction that allows Drop::drop to consume self by value**

Do you want to call a function that takes `self` by value in your `impl Drop`? Do you need the ability
to both destructure and drop your struct? Do you want a convenience type to temporarily give your 
values a new `Drop` implementation? This crate is for you.

## Safe, zero-cost API

`ConsumeOnDrop<T>` is a `#[repr(transparent)]` wrapper around `T`, and all provided operations are zero-cost.

`WithConsumer<T, Q>` is a thin wrapper around an ordered pair `(T, Q)`, and all its provided operations are zero-cost.

All public functions, methods, and traits in these APIs are completely safe.

## Implemented using minimal `unsafe` code

The implementation of `ConsumeOnDrop` has exactly 2 lines of `unsafe` code, both easy checked and tested with Miri.

The implementation of `WithConsumer` is completely safe (except insofar as it depends on the public API of 
`ConsumeOnDrop`).

## Consume your type by value on drop

```rust
use consume_on_drop::{ConsumeOnDrop, WithConsumer};

struct T;

fn consume_t(_t: T) {
    println!("We consumed T")
}

impl Consume for T {
    fn consume(self) {
        consume_t(self)
    }
}

fn main () {
    let t = ConsumeOnDrop::new(T);  // A thin wrapper around T which calls T::consume on drop
    drop(t);
    let t = WithConsumer::new(T, consume_t); // Alternately, we can explicitly equip a T with a consumer.
    drop(t);
}
```

## Write a struct that can be destructured and dropped

The following code doesn't compile.

```rust
struct MutRef<'a> {
    inner: &'a mut i32,
}

impl<'a> MyRef<'a> {
    pub fn new(val: &'a mut i32) -> Self {
        Self { inner: val }
    }
    
    pub fn into_inner(self) -> &'a mut i32 {
        self.inner
    }
}

impl<'a> Drop for MutRef<'a> {
    fn drop(&mut self) {
        *self.inner += 1;
    }
}
```

but we can make it work using `ConsumeOnDrop`:

```rust
use consume_on_drop::ConsumeOnDrop;

struct RawMut<'a> {
    inner: &'a mut i32,
}

impl<'a> Consume for RawMut<'a> {
    fn consume(self) {
        *self.inner += 1;
    }
}

struct MutRef<'a> {
    inner: ConsumeOnDrop<RawMut<'a>>,
}

impl<'a> MutRef<'a> {
    pub fn new(val: &'a mut i32) -> Self {
        Self { inner: ConsumeOnDrop::new(RawMut { inner: val })}
    }
    
    pub fn into_inner(self) -> &'a mut i32 {
        ConsumeOnDrop::into_inner(self.inner).inner
    }
}
```

Note that this is a zero-cost abstraction. We could achieve the same effect using `Option<RawMut>`, but this 
incurs run-time overhead and prevents us from using the null-pointer optimization on `Option<MutRef>`.

## Temporarily give your type a different drop implementation

Sometimes, you need to temporarily give your type a new `drop` implementation in case you return early or panic. 
Often, data may be left in an invalid state if a panic happens at the wrong time. To mark this, you may wish to "poison"
your data. 

```rust
use consume_on_drop::WithConsumer;

struct Data {
    string: Option<String>,
}

impl Data {
    fn new(str: String) -> Self {
        Self { string: Some(str) }
    }
    
    fn extend(&mut self, str: String) {
        self.string.as_mut().unwrap().extend(str.chars())
    }
    
    fn poison(&mut self) {
        self.string = None;
    }
}

fn produce_string() -> String {
    panic!("Oh no, we panicked!");
}

fn extend_produce(data: &mut Data) {
    let mut data = WithConsumer::new(data, Data::poison);
    data.extend(produce_string()); // if produce_string panics, we will drop data here and poison it
    WithConsumer::into_inner(data); // but if there's no panic, we will not poison.
}
```