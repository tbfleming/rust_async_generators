#[test]
fn no_state() {
    use gen::generate;

    assert_eq!(
        generate(|co| async move {
            co.yield_(4).await;
            co.yield_(3).await;
            co.yield_(2).await;
        })
        .collect::<Vec<_>>(),
        [4, 3, 2]
    );
}

#[test]
fn static_str_ref() {
    use gen::generate;

    assert_eq!(
        generate(|co| async move {
            co.yield_("First").await;
            co.yield_("Second").await;
            co.yield_("Third").await;
        })
        .collect::<Vec<_>>(),
        ["First", "Second", "Third"]
    );
}

#[test]
fn owned_string() {
    use gen::generate;

    assert_eq!(
        generate(|co| async move {
            co.yield_("First".to_owned()).await;
            co.yield_("Second".to_owned()).await;
            co.yield_("Third".to_owned()).await;
        })
        .collect::<Vec<_>>(),
        ["First".to_owned(), "Second".to_owned(), "Third".to_owned()]
    );
}

#[test]
fn repeat_none_at_end() {
    use gen::generate;

    let mut iter = generate(|co| async move {
        co.yield_(0).await;
        co.yield_(1).await;
        co.yield_(2).await;
    });

    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}

#[test]
fn local_var() {
    use gen::generate;

    assert_eq!(
        generate(|co| async move {
            for i in 0..10 {
                co.yield_(i).await;
            }
        })
        .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
}

#[test]
fn mut_ref() {
    use gen::generate;

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
    assert_eq!(i, 45);
}

#[test]
fn move_iter_to_thread() {
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
}
