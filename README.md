# Generators for Rust

I recently ran across [genawaiter](https://docs.rs/genawaiter/latest/genawaiter/) and similar crates. I decided to try my own clean-room implementation of a generator library as a personal challenge. Here are my goals (all met):

* Base its API around `async/await`.
* No macros in its API.
* Don't use threads in its implementation or require them in its API.
* No `unsafe` code; I wanted to see if the borrow checker would get in the way. It did for my first several attempts, but I eventually landed on an approach that made it happy. As a side effect, the source for the final library is more readable than my prior attempts.
* Bonus goal: allow the generator to safely move between threads.
* I looked at some of `genawaiter's` examples, but not its source.

This isn't production ready and likely never will be (that'd be needlessly redundant), so I don't intend to push this to https://crates.io/.

## Example Usage

```rust
use gen::generate;

// Generate some items
let iter = generate(|co| async move {
    co.yield_("First").await;
    co.yield_("Second").await;
    co.yield_("Third").await;
});

// "First", "Second", "Third"
for s in iter {
    println!("s = {s}");
}
```

This doesn't use any containers; each call to `yield_` unblocks a waiting call to `iter.next()`. See the top of [src/lib.rs](src/lib.rs) for a little more detail.

## Manipulating an infinite set

```rust
use gen::generate;

// Generate [0, 1, 2, 3, ...]
let iter = generate(|co| async move {
    for i in 0.. {
        co.yield_(i).await;
    }
});

// 0, 5, 10, 15, 20, 25, 30, 35, 40, 45
for j in iter.step_by(5).take(10) {
    println!("j = {j}");
}

// The for loop consumed the iterator. This canceled the
// infinite loop in the async block after it yielded `45`
// and dropped (cleaned up) its state.
```

## Borrowing within the async block

The library had to get lifetimes just right to make the borrow checker happy with this example.

```rust
use gen::generate;

// The async block borrows `i` via `ir`
let mut i = 0;
let ir = &mut i;

assert_eq!(
    generate(|co| async move {
        loop {
            co.yield_(*ir).await;
            *ir += 1;
        }
    })
    .step_by(5)
    .take(10)
    .collect::<Vec<_>>(),
    [0, 5, 10, 15, 20, 25, 30, 35, 40, 45]
);

// Verify the async block modified `i`
assert_eq!(i, 45);
```

## Move an active generator between threads

This capability only required minor changes to the library implementation (replacing `Rc` with `Arc` and `RefCell` with `Mutex`). Since the library has no unsafe code and uses no external crates, I can be confident it [has no data races or other forms of UB](https://blog.rust-lang.org/2015/04/10/Fearless-Concurrency.html).

```rust
use gen::generate;
use std::thread;

// The async block borrows `i` via `ir`
let mut i = 0;
let ir = &mut i;

// Create the generator on the main thread. Any thread may
// create it. Any thread may use it.
let mut iter = generate(|co| async move {
    loop {
        co.yield_(*ir).await;
        *ir += 1;
    }
});

// Get some items from the generator on the main thread.
for index in 0..5 {
    assert_eq!(iter.next(), Some(index));
}

// Borrow the iterator on another thread and use it there.
thread::scope(|s| {
    s.spawn(|| {
        for i in 5..10 {
            assert_eq!(iter.next(), Some(i));
        }
    });
});

// Get some more items on the main thread.
for index in 10..15 {
    assert_eq!(iter.next(), Some(index));
}

// Move the iterator to another thread and use it there.
// (notice the 2 moves)
thread::scope(move |s| {
    s.spawn(move || {
        for index in 15..20 {
            assert_eq!(iter.next(), Some(index));
        }
    });
});

// Verify the async block modified `i`
assert_eq!(i, 19);
```
