//! Streams
//!
//! This module contains a number of functions for working with `Stream`s,
//! including the `StreamExt` trait which adds methods to `Stream` types.

use core::pin::Pin;
use futures_core::future::Future;
use futures_core::stream::{FusedStream, Stream, TryStream};
use futures_core::task::{Context, Poll};
use futures_sink::Sink;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use crate::future::Either;

mod iter;
pub use self::iter::{iter, Iter};

mod repeat;
pub use self::repeat::{repeat, Repeat};

mod chain;
pub use self::chain::Chain;

mod collect;
pub use self::collect::Collect;

mod concat;
pub use self::concat::Concat;

mod empty;
pub use self::empty::{empty, Empty};

mod enumerate;
pub use self::enumerate::Enumerate;

mod filter;
pub use self::filter::Filter;

mod filter_map;
pub use self::filter_map::FilterMap;

mod flatten;
pub use self::flatten::Flatten;

mod fold;
pub use self::fold::Fold;

mod forward;
pub use self::forward::Forward;

mod for_each;
pub use self::for_each::ForEach;

mod fuse;
pub use self::fuse::Fuse;

mod into_future;
pub use self::into_future::StreamFuture;

mod inspect;
pub(crate) use self::inspect::inspect; // used by `TryStreamExt::{inspect_ok, inspect_err}`
pub use self::inspect::Inspect;

mod map;
pub use self::map::Map;

mod next;
pub use self::next::Next;

mod select_next_some;
pub use self::select_next_some::SelectNextSome;

mod once;
pub use self::once::{once, Once};

mod peek;
pub use self::peek::Peekable;

mod pending;
pub use self::pending::{pending, Pending};

mod poll_fn;
pub use self::poll_fn::{poll_fn, PollFn};

mod select;
pub use self::select::{select, Select};

mod skip;
pub use self::skip::Skip;

mod skip_while;
pub use self::skip_while::SkipWhile;

mod take;
pub use self::take::Take;

mod take_while;
pub use self::take_while::TakeWhile;

mod then;
pub use self::then::Then;

mod unfold;
pub use self::unfold::{unfold, Unfold};

mod zip;
pub use self::zip::Zip;

#[cfg(feature = "alloc")]
mod chunks;
#[cfg(feature = "alloc")]
pub use self::chunks::Chunks;

cfg_target_has_atomic! {
    #[cfg(feature = "alloc")]
    mod buffer_unordered;
    #[cfg(feature = "alloc")]
    pub use self::buffer_unordered::BufferUnordered;

    #[cfg(feature = "alloc")]
    mod buffered;
    #[cfg(feature = "alloc")]
    pub use self::buffered::Buffered;

    #[cfg(feature = "alloc")]
    mod for_each_concurrent;
    #[cfg(feature = "alloc")]
    pub use self::for_each_concurrent::ForEachConcurrent;

    #[cfg(feature = "alloc")]
    mod futures_ordered;
    #[cfg(feature = "alloc")]
    pub use self::futures_ordered::FuturesOrdered;

    #[cfg(feature = "alloc")]
    pub mod futures_unordered;
    #[cfg(feature = "alloc")]
    #[doc(inline)]
    pub use self::futures_unordered::FuturesUnordered;

    #[cfg(feature = "alloc")]
    mod split;
    #[cfg(feature = "alloc")]
    pub use self::split::{SplitStream, SplitSink, ReuniteError};

    #[cfg(feature = "alloc")]
    mod select_all;
    #[cfg(feature = "alloc")]
    pub use self::select_all::{select_all, SelectAll};
}

#[cfg(feature = "std")]
mod catch_unwind;
#[cfg(feature = "std")]
pub use self::catch_unwind::CatchUnwind;

impl<T: ?Sized> StreamExt for T where T: Stream {}

/// An extension trait for `Stream`s that provides a variety of convenient
/// combinator functions.
pub trait StreamExt: Stream {
    /// Creates a future that resolves to the next item in the stream.
    ///
    /// Note that because `next` doesn't take ownership over the stream,
    /// the [`Stream`] type must be [`Unpin`]. If you want to use `next` with a
    /// [`!Unpin`](Unpin) stream, you'll first have to pin the stream. This can
    /// be done by boxing the stream using [`Box::pin`] or
    /// pinning it to the stack using the `pin_mut!` macro from the `pin_utils`
    /// crate.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let mut stream = stream::iter(1..=3);
    ///
    /// assert_eq!(stream.next().await, Some(1));
    /// assert_eq!(stream.next().await, Some(2));
    /// assert_eq!(stream.next().await, Some(3));
    /// assert_eq!(stream.next().await, None);
    /// # });
    /// ```
    fn next(&mut self) -> Next<'_, Self>
        where Self: Unpin,
    {
        Next::new(self)
    }

    /// Converts this stream into a future of `(next_item, tail_of_stream)`.
    /// If the stream terminates, then the next item is [`None`].
    ///
    /// The returned future can be used to compose streams and futures together
    /// by placing everything into the "world of futures".
    ///
    /// Note that because `into_future` moves the stream, the [`Stream`] type
    /// must be [`Unpin`]. If you want to use `into_future` with a
    /// [`!Unpin`](Unpin) stream, you'll first have to pin the stream. This can
    /// be done by boxing the stream using [`Box::pin`] or
    /// pinning it to the stack using the `pin_mut!` macro from the `pin_utils`
    /// crate.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=3);
    ///
    /// let (item, stream) = stream.into_future().await;
    /// assert_eq!(Some(1), item);
    ///
    /// let (item, stream) = stream.into_future().await;
    /// assert_eq!(Some(2), item);
    /// # });
    /// ```
    fn into_future(self) -> StreamFuture<Self>
        where Self: Sized + Unpin,
    {
        StreamFuture::new(self)
    }

    /// Maps this stream's items to a different type, returning a new stream of
    /// the resulting type.
    ///
    /// The provided closure is executed over all elements of this stream as
    /// they are made available. It is executed inline with calls to
    /// [`poll_next`](Stream::poll_next).
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it, similar to the existing `map` methods in the
    /// standard library.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=3);
    /// let stream = stream.map(|x| x + 3);
    ///
    /// assert_eq!(vec![4, 5, 6], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn map<T, F>(self, f: F) -> Map<Self, F>
        where F: FnMut(Self::Item) -> T,
              Self: Sized
    {
        Map::new(self, f)
    }

    /// Creates a stream which gives the current iteration count as well as
    /// the next value.
    ///
    /// The stream returned yields pairs `(i, val)`, where `i` is the
    /// current index of iteration and `val` is the value returned by the
    /// stream.
    ///
    /// `enumerate()` keeps its count as a [`usize`]. If you want to count by a
    /// different sized integer, the [`zip`](StreamExt::zip) function provides similar
    /// functionality.
    ///
    /// # Overflow Behavior
    ///
    /// The method does no guarding against overflows, so enumerating more than
    /// [`usize::max_value()`] elements either produces the wrong result or panics. If
    /// debug assertions are enabled, a panic is guaranteed.
    ///
    /// # Panics
    ///
    /// The returned stream might panic if the to-be-returned index would
    /// overflow a [`usize`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(vec!['a', 'b', 'c']);
    ///
    /// let mut stream = stream.enumerate();
    ///
    /// assert_eq!(stream.next().await, Some((0, 'a')));
    /// assert_eq!(stream.next().await, Some((1, 'b')));
    /// assert_eq!(stream.next().await, Some((2, 'c')));
    /// assert_eq!(stream.next().await, None);
    /// # });
    /// ```
    fn enumerate(self) -> Enumerate<Self>
        where Self: Sized,
    {
        Enumerate::new(self)
    }

    /// Filters the values produced by this stream according to the provided
    /// asynchronous predicate.
    ///
    /// As values of this stream are made available, the provided predicate `f`
    /// will be run against them. If the predicate returns a `Future` which
    /// resolves to `true`, then the stream will yield the value, but if the
    /// predicate returns a `Future` which resolves to `false`, then the value
    /// will be discarded and the next value will be produced.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it, similar to the existing `filter` methods in the
    /// standard library.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10);
    /// let evens = stream.filter(|x| future::ready(x % 2 == 0));
    ///
    /// assert_eq!(vec![2, 4, 6, 8, 10], evens.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn filter<Fut, F>(self, f: F) -> Filter<Self, Fut, F>
        where F: FnMut(&Self::Item) -> Fut,
              Fut: Future<Output = bool>,
              Self: Sized,
    {
        Filter::new(self, f)
    }

    /// Filters the values produced by this stream while simultaneously mapping
    /// them to a different type according to the provided asynchronous closure.
    ///
    /// As values of this stream are made available, the provided function will
    /// be run on them. If the future returned by the predicate `f` resolves to
    /// [`Some(item)`](Some) then the stream will yield the value `item`, but if
    /// it resolves to [`None`] then the next value will be produced.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it, similar to the existing `filter_map` methods in
    /// the standard library.
    ///
    /// # Examples
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10);
    /// let evens = stream.filter_map(|x| {
    ///     let ret = if x % 2 == 0 { Some(x + 1) } else { None };
    ///     future::ready(ret)
    /// });
    ///
    /// assert_eq!(vec![3, 5, 7, 9, 11], evens.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn filter_map<Fut, T, F>(self, f: F) -> FilterMap<Self, Fut, F>
        where F: FnMut(Self::Item) -> Fut,
              Fut: Future<Output = Option<T>>,
              Self: Sized,
    {
        FilterMap::new(self, f)
    }

    /// Computes from this stream's items new items of a different type using
    /// an asynchronous closure.
    ///
    /// The provided closure `f` will be called with an `Item` once a value is
    /// ready, it returns a future which will then be run to completion
    /// to produce the next value on this stream.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=3);
    /// let stream = stream.then(|x| future::ready(x + 3));
    ///
    /// assert_eq!(vec![4, 5, 6], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn then<Fut, F>(self, f: F) -> Then<Self, Fut, F>
        where F: FnMut(Self::Item) -> Fut,
              Fut: Future,
              Self: Sized
    {
        Then::new(self, f)
    }

    /// Collect all of the values of this stream into a vector, returning a
    /// future representing the result of that computation.
    ///
    /// The returned future will be resolved when the stream terminates.
    ///
    /// # Examples
    ///
    // TODO: https://github.com/rust-lang-nursery/futures-rs/issues/1657
    /// ```ignore
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::mpsc;
    /// use futures::stream::StreamExt;
    /// use std::thread;
    ///
    /// let (tx, rx) = mpsc::unbounded();
    ///
    /// thread::spawn(move || {
    ///     for i in (1..=5) {
    ///         tx.unbounded_send(i).unwrap();
    ///     }
    /// });
    ///
    /// let output = rx.collect::<Vec<i32>>().await;
    /// assert_eq!(output, vec![1, 2, 3, 4, 5]);
    /// # });
    /// ```
    fn collect<C: Default + Extend<Self::Item>>(self) -> Collect<Self, C>
        where Self: Sized
    {
        Collect::new(self)
    }

    /// Concatenate all items of a stream into a single extendable
    /// destination, returning a future representing the end result.
    ///
    /// This combinator will extend the first item with the contents
    /// of all the subsequent results of the stream. If the stream is
    /// empty, the default value will be returned.
    ///
    /// Works with all collections that implement the
    /// [`Extend`](std::iter::Extend) trait.
    ///
    /// # Examples
    ///
    // TODO: https://github.com/rust-lang-nursery/futures-rs/issues/1657
    /// ```ignore
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::mpsc;
    /// use futures::stream::StreamExt;
    /// use std::thread;
    ///
    /// let (tx, rx) = mpsc::unbounded();
    ///
    /// thread::spawn(move || {
    ///     for i in (0..3).rev() {
    ///         let n = i * 3;
    ///         tx.unbounded_send(vec![n + 1, n + 2, n + 3]).unwrap();
    ///     }
    /// });
    ///
    /// let result = rx.concat().await;
    ///
    /// assert_eq!(result, vec![7, 8, 9, 4, 5, 6, 1, 2, 3]);
    /// # });
    /// ```
    fn concat(self) -> Concat<Self>
    where Self: Sized,
          Self::Item: Extend<<<Self as Stream>::Item as IntoIterator>::Item> +
                      IntoIterator + Default,
    {
        Concat::new(self)
    }

    /// Execute an accumulating asynchronous computation over a stream,
    /// collecting all the values into one final result.
    ///
    /// This combinator will accumulate all values returned by this stream
    /// according to the closure provided. The initial state is also provided to
    /// this method and then is returned again by each execution of the closure.
    /// Once the entire stream has been exhausted the returned future will
    /// resolve to this value.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let number_stream = stream::iter(0..6);
    /// let sum = number_stream.fold(0, |acc, x| future::ready(acc + x));
    /// assert_eq!(sum.await, 15);
    /// # });
    /// ```
    fn fold<T, Fut, F>(self, init: T, f: F) -> Fold<Self, Fut, T, F>
        where F: FnMut(T, Self::Item) -> Fut,
              Fut: Future<Output = T>,
              Self: Sized
    {
        Fold::new(self, f, init)
    }

    /// Flattens a stream of streams into just one continuous stream.
    ///
    /// # Examples
    ///
    // TODO: https://github.com/rust-lang-nursery/futures-rs/issues/1657
    /// ```ignore
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::mpsc;
    /// use futures::stream::StreamExt;
    /// use std::thread;
    ///
    /// let (tx1, rx1) = mpsc::unbounded();
    /// let (tx2, rx2) = mpsc::unbounded();
    /// let (tx3, rx3) = mpsc::unbounded();
    ///
    /// thread::spawn(move || {
    ///     tx1.unbounded_send(1).unwrap();
    ///     tx1.unbounded_send(2).unwrap();
    /// });
    /// thread::spawn(move || {
    ///     tx2.unbounded_send(3).unwrap();
    ///     tx2.unbounded_send(4).unwrap();
    /// });
    /// thread::spawn(move || {
    ///     tx3.unbounded_send(rx1).unwrap();
    ///     tx3.unbounded_send(rx2).unwrap();
    /// });
    ///
    /// let output = rx3.flatten().collect::<Vec<i32>>().await;
    /// assert_eq!(output, vec![1, 2, 3, 4]);
    /// # });
    /// ```
    fn flatten(self) -> Flatten<Self>
        where Self::Item: Stream,
              Self: Sized
    {
        Flatten::new(self)
    }

    /// Skip elements on this stream while the provided asynchronous predicate
    /// resolves to `true`.
    ///
    /// This function, like `Iterator::skip_while`, will skip elements on the
    /// stream until the predicate `f` resolves to `false`. Once one element
    /// returns false all future elements will be returned from the underlying
    /// stream.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10);
    ///
    /// let stream = stream.skip_while(|x| future::ready(*x <= 5));
    ///
    /// assert_eq!(vec![6, 7, 8, 9, 10], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn skip_while<Fut, F>(self, f: F) -> SkipWhile<Self, Fut, F>
        where F: FnMut(&Self::Item) -> Fut,
              Fut: Future<Output = bool>,
              Self: Sized
    {
        SkipWhile::new(self, f)
    }

    /// Take elements from this stream while the provided asynchronous predicate
    /// resolves to `true`.
    ///
    /// This function, like `Iterator::take_while`, will take elements from the
    /// stream until the predicate `f` resolves to `false`. Once one element
    /// returns false it will always return that the stream is done.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10);
    ///
    /// let stream = stream.take_while(|x| future::ready(*x <= 5));
    ///
    /// assert_eq!(vec![1, 2, 3, 4, 5], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn take_while<Fut, F>(self, f: F) -> TakeWhile<Self, Fut, F>
        where F: FnMut(&Self::Item) -> Fut,
              Fut: Future<Output = bool>,
              Self: Sized
    {
        TakeWhile::new(self, f)
    }

    /// Runs this stream to completion, executing the provided asynchronous
    /// closure for each element on the stream.
    ///
    /// The closure provided will be called for each item this stream produces,
    /// yielding a future. That future will then be executed to completion
    /// before moving on to the next item.
    ///
    /// The returned value is a `Future` where the `Output` type is `()`; it is
    /// executed entirely for its side effects.
    ///
    /// To process each item in the stream and produce another stream instead
    /// of a single future, use `then` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let mut x = 0;
    ///
    /// {
    ///     let fut = stream::repeat(1).take(3).for_each(|item| {
    ///         x += item;
    ///         future::ready(())
    ///     });
    ///     fut.await;
    /// }
    ///
    /// assert_eq!(x, 3);
    /// # });
    /// ```
    fn for_each<Fut, F>(self, f: F) -> ForEach<Self, Fut, F>
        where F: FnMut(Self::Item) -> Fut,
              Fut: Future<Output = ()>,
              Self: Sized
    {
        ForEach::new(self, f)
    }

    /// Runs this stream to completion, executing the provided asynchronous
    /// closure for each element on the stream concurrently as elements become
    /// available.
    ///
    /// This is similar to [`StreamExt::for_each`], but the futures
    /// produced by the closure are run concurrently (but not in parallel--
    /// this combinator does not introduce any threads).
    ///
    /// The closure provided will be called for each item this stream produces,
    /// yielding a future. That future will then be executed to completion
    /// concurrently with the other futures produced by the closure.
    ///
    /// The first argument is an optional limit on the number of concurrent
    /// futures. If this limit is not `None`, no more than `limit` futures
    /// will be run concurrently. The `limit` argument is of type
    /// `Into<Option<usize>>`, and so can be provided as either `None`,
    /// `Some(10)`, or just `10`. Note: a limit of zero is interpreted as
    /// no limit at all, and will have the same result as passing in `None`.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::oneshot;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let (tx1, rx1) = oneshot::channel();
    /// let (tx2, rx2) = oneshot::channel();
    /// let (tx3, rx3) = oneshot::channel();
    ///
    /// let fut = stream::iter(vec![rx1, rx2, rx3]).for_each_concurrent(
    ///     /* limit */ 2,
    ///     async move |rx| {
    ///         rx.await.unwrap();
    ///     }
    /// );
    /// tx1.send(()).unwrap();
    /// tx2.send(()).unwrap();
    /// tx3.send(()).unwrap();
    /// fut.await;
    /// # })
    /// ```
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn for_each_concurrent<Fut, F>(
        self,
        limit: impl Into<Option<usize>>,
        f: F,
    ) -> ForEachConcurrent<Self, Fut, F>
        where F: FnMut(Self::Item) -> Fut,
              Fut: Future<Output = ()>,
              Self: Sized,
    {
        ForEachConcurrent::new(self, limit.into(), f)
    }

    /// Creates a new stream of at most `n` items of the underlying stream.
    ///
    /// Once `n` items have been yielded from this stream then it will always
    /// return that the stream is done.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10).take(3);
    ///
    /// assert_eq!(vec![1, 2, 3], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn take(self, n: u64) -> Take<Self>
        where Self: Sized
    {
        Take::new(self, n)
    }

    /// Creates a new stream which skips `n` items of the underlying stream.
    ///
    /// Once `n` items have been skipped from this stream then it will always
    /// return the remaining items on this stream.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(1..=10).skip(5);
    ///
    /// assert_eq!(vec![6, 7, 8, 9, 10], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn skip(self, n: u64) -> Skip<Self>
        where Self: Sized
    {
        Skip::new(self, n)
    }

    /// Fuse a stream such that [`poll_next`](Stream::poll_next) will never
    /// again be called once it has finished. This method can be used t turn
    /// any `Stream` into a `FusedStream`.
    ///
    /// Normally, once a stream has returned [`None`] from
    /// [`poll_next`](Stream::poll_next) any further calls could exhibit bad
    /// behavior such as block forever, panic, never return, etc. If it is known
    /// that [`poll_next`](Stream::poll_next) may be called after stream
    /// has already finished, then this method can be used to ensure that it has
    /// defined semantics.
    ///
    /// The [`poll_next`](Stream::poll_next) method of a `fuse`d stream
    /// is guaranteed to return [`None`] after the underlying stream has
    /// finished.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on_stream;
    /// use futures::stream::{self, StreamExt};
    /// use futures::task::Poll;
    ///
    /// let mut x = 0;
    /// let stream = stream::poll_fn(|_| {
    ///     x += 1;
    ///     match x {
    ///         0..=2 => Poll::Ready(Some(x)),
    ///         3 => Poll::Ready(None),
    ///         _ => panic!("should not happen")
    ///     }
    /// }).fuse();
    ///
    /// let mut iter = block_on_stream(stream);
    /// assert_eq!(Some(1), iter.next());
    /// assert_eq!(Some(2), iter.next());
    /// assert_eq!(None, iter.next());
    /// assert_eq!(None, iter.next());
    /// // ...
    /// ```
    fn fuse(self) -> Fuse<Self>
        where Self: Sized
    {
        Fuse::new(self)
    }

    /// Borrows a stream, rather than consuming it.
    ///
    /// This is useful to allow applying stream adaptors while still retaining
    /// ownership of the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let mut stream = stream::iter(1..5);
    ///
    /// let sum = stream.by_ref()
    ///                 .take(2)
    ///                 .fold(0, |a, b| future::ready(a + b))
    ///                 .await;
    /// assert_eq!(sum, 3);
    ///
    /// // You can use the stream again
    /// let sum = stream.take(2)
    ///                 .fold(0, |a, b| future::ready(a + b))
    ///                 .await;
    /// assert_eq!(sum, 7);
    /// # });
    /// ```
    fn by_ref(&mut self) -> &mut Self {
        self
    }

    /// Catches unwinding panics while polling the stream.
    ///
    /// Caught panic (if any) will be the last element of the resulting stream.
    ///
    /// In general, panics within a stream can propagate all the way out to the
    /// task level. This combinator makes it possible to halt unwinding within
    /// the stream itself. It's most commonly used within task executors. This
    /// method should not be used for error handling.
    ///
    /// Note that this method requires the `UnwindSafe` bound from the standard
    /// library. This isn't always applied automatically, and the standard
    /// library provides an `AssertUnwindSafe` wrapper type to apply it
    /// after-the fact. To assist using this method, the [`Stream`] trait is
    /// also implemented for `AssertUnwindSafe<St>` where `St` implements
    /// [`Stream`].
    ///
    /// This method is only available when the `std` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(vec![Some(10), None, Some(11)]);
    /// // Panic on second element
    /// let stream_panicking = stream.map(|o| o.unwrap());
    /// // Collect all the results
    /// let stream = stream_panicking.catch_unwind();
    ///
    /// let results: Vec<Result<i32, _>> = stream.collect().await;
    /// match results[0] {
    ///     Ok(10) => {}
    ///     _ => panic!("unexpected result!"),
    /// }
    /// assert!(results[1].is_err());
    /// assert_eq!(results.len(), 2);
    /// # });
    /// ```
    #[cfg(feature = "std")]
    fn catch_unwind(self) -> CatchUnwind<Self>
        where Self: Sized + std::panic::UnwindSafe
    {
        CatchUnwind::new(self)
    }

    /// Wrap the stream in a Box, pinning it.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    #[cfg(feature = "alloc")]
    fn boxed(self) -> Pin<Box<Self>>
        where Self: Sized
    {
        Box::pin(self)
    }

    /// An adaptor for creating a buffered list of pending futures.
    ///
    /// If this stream's item can be converted into a future, then this adaptor
    /// will buffer up to at most `n` futures and then return the outputs in the
    /// same order as the underlying stream. No more than `n` futures will be
    /// buffered at any point in time, and less than `n` may also be buffered
    /// depending on the state of each future.
    ///
    /// The returned stream will be a stream of each future's output.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn buffered(self, n: usize) -> Buffered<Self>
        where Self::Item: Future,
              Self: Sized
    {
        Buffered::new(self, n)
    }

    /// An adaptor for creating a buffered list of pending futures (unordered).
    ///
    /// If this stream's item can be converted into a future, then this adaptor
    /// will buffer up to `n` futures and then return the outputs in the order
    /// in which they complete. No more than `n` futures will be buffered at
    /// any point in time, and less than `n` may also be buffered depending on
    /// the state of each future.
    ///
    /// The returned stream will be a stream of each future's output.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::oneshot;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let (send_one, recv_one) = oneshot::channel();
    /// let (send_two, recv_two) = oneshot::channel();
    ///
    /// let stream_of_futures = stream::iter(vec![recv_one, recv_two]);
    /// let mut buffered = stream_of_futures.buffer_unordered(10);
    ///
    /// send_two.send(2i32)?;
    /// assert_eq!(buffered.next().await, Some(Ok(2i32)));
    ///
    /// send_one.send(1i32)?;
    /// assert_eq!(buffered.next().await, Some(Ok(1i32)));
    ///
    /// assert_eq!(buffered.next().await, None);
    /// # Ok::<(), i32>(()) }).unwrap();
    /// ```
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn buffer_unordered(self, n: usize) -> BufferUnordered<Self>
        where Self::Item: Future,
              Self: Sized
    {
        BufferUnordered::new(self, n)
    }

    /// An adapter for zipping two streams together.
    ///
    /// The zipped stream waits for both streams to produce an item, and then
    /// returns that pair. If either stream ends then the zipped stream will
    /// also end.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream1 = stream::iter(1..=3);
    /// let stream2 = stream::iter(5..=10);
    ///
    /// let vec = stream1.zip(stream2)
    ///                  .collect::<Vec<_>>()
    ///                  .await;
    /// assert_eq!(vec![(1, 5), (2, 6), (3, 7)], vec);
    /// # });
    /// ```
    ///
    fn zip<St>(self, other: St) -> Zip<Self, St>
        where St: Stream,
              Self: Sized,
    {
        Zip::new(self, other)
    }

    /// Adapter for chaining two stream.
    ///
    /// The resulting stream emits elements from the first stream, and when
    /// first stream reaches the end, emits the elements from the second stream.
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream1 = stream::iter(vec![Ok(10), Err(false)]);
    /// let stream2 = stream::iter(vec![Err(true), Ok(20)]);
    ///
    /// let stream = stream1.chain(stream2);
    ///
    /// let result: Vec<_> = stream.collect().await;
    /// assert_eq!(result, vec![
    ///     Ok(10),
    ///     Err(false),
    ///     Err(true),
    ///     Ok(20),
    /// ]);
    /// # });
    /// ```
    fn chain<St>(self, other: St) -> Chain<Self, St>
        where St: Stream<Item = Self::Item>,
              Self: Sized
    {
        Chain::new(self, other)
    }

    /// Creates a new stream which exposes a `peek` method.
    ///
    /// Calling `peek` returns a reference to the next item in the stream.
    fn peekable(self) -> Peekable<Self>
        where Self: Sized
    {
        Peekable::new(self)
    }

    /// An adaptor for chunking up items of the stream inside a vector.
    ///
    /// This combinator will attempt to pull items from this stream and buffer
    /// them into a local vector. At most `capacity` items will get buffered
    /// before they're yielded from the returned stream.
    ///
    /// Note that the vectors returned from this iterator may not always have
    /// `capacity` elements. If the underlying stream ended and only a partial
    /// vector was created, it'll be returned. Additionally if an error happens
    /// from the underlying stream then the currently buffered items will be
    /// yielded.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Panics
    ///
    /// This method will panic if `capacity` is zero.
    #[cfg(feature = "alloc")]
    fn chunks(self, capacity: usize) -> Chunks<Self>
        where Self: Sized
    {
        Chunks::new(self, capacity)
    }

    /// A future that completes after the given stream has been fully processed
    /// into the sink and the sink has been flushed and closed.
    ///
    /// This future will drive the stream to keep producing items until it is
    /// exhausted, sending each item to the sink. It will complete once the
    /// stream is exhausted, the sink has received and flushed all items, and
    /// the sink is closed. Note that neither the original stream nor provided
    /// sink will be output by this future.  Pass the sink by `Pin<&mut S>`
    /// (for example, via `forward(&mut sink)` inside an `async` fn/block) in
    /// order to preserve access to the Sink.
    fn forward<S>(self, sink: S) -> Forward<Self, S>
    where
        S: Sink<<Self as TryStream>::Ok>,
        Self: TryStream<Error = S::SinkError> + Sized,
    {
        Forward::new(self, sink)
    }

    /// Splits this `Stream + Sink` object into separate `Stream` and `Sink`
    /// objects.
    ///
    /// This can be useful when you want to split ownership between tasks, or
    /// allow direct interaction between the two objects (e.g. via
    /// `Sink::send_all`).
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn split<Item>(self) -> (SplitSink<Self, Item>, SplitStream<Self>)
        where Self: Sink<Item> + Sized
    {
        split::split(self)
    }

    /// Do something with each item of this stream, afterwards passing it on.
    ///
    /// This is similar to the `Iterator::inspect` method in the standard
    /// library where it allows easily inspecting each value as it passes
    /// through the stream, for example to debug what's going on.
    fn inspect<F>(self, f: F) -> Inspect<Self, F>
        where F: FnMut(&Self::Item),
              Self: Sized,
    {
        Inspect::new(self, f)
    }

    /// Wrap this stream in an `Either` stream, making it the left-hand variant
    /// of that `Either`.
    ///
    /// This can be used in combination with the `right_stream` method to write `if`
    /// statements that evaluate to different streams in different branches.
    fn left_stream<B>(self) -> Either<Self, B>
        where B: Stream<Item = Self::Item>,
              Self: Sized
    {
        Either::Left(self)
    }

    /// Wrap this stream in an `Either` stream, making it the right-hand variant
    /// of that `Either`.
    ///
    /// This can be used in combination with the `left_stream` method to write `if`
    /// statements that evaluate to different streams in different branches.
    fn right_stream<B>(self) -> Either<B, Self>
        where B: Stream<Item = Self::Item>,
              Self: Sized
    {
        Either::Right(self)
    }

    /// A convenience method for calling [`Stream::poll_next`] on [`Unpin`]
    /// stream types.
    fn poll_next_unpin(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>>
    where Self: Unpin
    {
        Pin::new(self).poll_next(cx)
    }

    /// Returns a [`Future`] that resolves when the next item in this stream is
    /// ready.
    ///
    /// This is similar to the [`next`][StreamExt::next] method, but it won't
    /// resolve to [`None`] if used on an empty [`Stream`]. Instead, the
    /// returned future type will return `true` from
    /// [`FusedFuture::is_terminated`][] when the [`Stream`] is empty, allowing
    /// [`select_next_some`][StreamExt::select_next_some] to be easily used with
    /// the [`select!`] macro.
    ///
    /// If the future is polled after this [`Stream`] is empty it will panic.
    /// Using the future with a [`FusedFuture`][]-aware primitive like the
    /// [`select!`] macro will prevent this.
    ///
    /// [`FusedFuture`]: futures_core::future::FusedFuture
    /// [`FusedFuture::is_terminated`]: futures_core::future::FusedFuture::is_terminated
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::{future, select};
    /// use futures::stream::{StreamExt, FuturesUnordered};
    ///
    /// let mut fut = future::ready(1);
    /// let mut async_tasks = FuturesUnordered::new();
    /// let mut total = 0;
    /// loop {
    ///     select! {
    ///         num = fut => {
    ///             // First, the `ready` future completes.
    ///             total += num;
    ///             // Then we spawn a new task onto `async_tasks`,
    ///             async_tasks.push(async { 5 });
    ///         },
    ///         // On the next iteration of the loop, the task we spawned
    ///         // completes.
    ///         num = async_tasks.select_next_some() => {
    ///             total += num;
    ///         }
    ///         // Finally, both the `ready` future and `async_tasks` have
    ///         // finished, so we enter the `complete` branch.
    ///         complete => break,
    ///     }
    /// }
    /// assert_eq!(total, 6);
    /// # });
    /// ```
    fn select_next_some(&mut self) -> SelectNextSome<'_, Self> where Self: Unpin + FusedStream {
        SelectNextSome::new(self)
    }
}
