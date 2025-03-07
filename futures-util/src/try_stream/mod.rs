//! Streams
//!
//! This module contains a number of functions for working with `Streams`s
//! that return `Result`s, allowing for short-circuiting computations.

use core::pin::Pin;
use futures_core::future::{Future, TryFuture};
use futures_core::stream::TryStream;
use futures_core::task::{Context, Poll};

#[cfg(feature = "compat")]
use crate::compat::Compat;

mod and_then;
pub use self::and_then::AndThen;

mod err_into;
pub use self::err_into::ErrInto;

mod inspect_ok;
pub use self::inspect_ok::InspectOk;

mod inspect_err;
pub use self::inspect_err::InspectErr;

mod into_stream;
pub use self::into_stream::IntoStream;

mod map_ok;
pub use self::map_ok::MapOk;

mod map_err;
pub use self::map_err::MapErr;

mod or_else;
pub use self::or_else::OrElse;

mod try_next;
pub use self::try_next::TryNext;

mod try_for_each;
pub use self::try_for_each::TryForEach;

mod try_filter;
pub use self::try_filter::TryFilter;

mod try_filter_map;
pub use self::try_filter_map::TryFilterMap;

mod try_collect;
pub use self::try_collect::TryCollect;

mod try_concat;
pub use self::try_concat::TryConcat;

mod try_fold;
pub use self::try_fold::TryFold;

mod try_skip_while;
pub use self::try_skip_while::TrySkipWhile;

cfg_target_has_atomic! {
    #[cfg(feature = "alloc")]
    mod try_buffer_unordered;
    #[cfg(feature = "alloc")]
    pub use self::try_buffer_unordered::TryBufferUnordered;

    #[cfg(feature = "alloc")]
    mod try_for_each_concurrent;
    #[cfg(feature = "alloc")]
    pub use self::try_for_each_concurrent::TryForEachConcurrent;
}

#[cfg(feature = "std")]
mod into_async_read;
#[cfg(feature = "std")]
pub use self::into_async_read::IntoAsyncRead;

impl<S: ?Sized + TryStream> TryStreamExt for S {}

/// Adapters specific to `Result`-returning streams
pub trait TryStreamExt: TryStream {
    /// Wraps the current stream in a new stream which converts the error type
    /// into the one provided.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let mut stream =
    ///     stream::iter(vec![Ok(()), Err(5i32)])
    ///         .err_into::<i64>();
    ///
    /// assert_eq!(stream.try_next().await, Ok(Some(())));
    /// assert_eq!(stream.try_next().await, Err(5i64));
    /// # })
    /// ```
    fn err_into<E>(self) -> ErrInto<Self, E>
    where
        Self: Sized,
        Self::Error: Into<E>
    {
        ErrInto::new(self)
    }

    /// Wraps the current stream in a new stream which maps the success value
    /// using the provided closure.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let mut stream =
    ///     stream::iter(vec![Ok(5), Err(0)])
    ///         .map_ok(|x| x + 2);
    ///
    /// assert_eq!(stream.try_next().await, Ok(Some(7)));
    /// assert_eq!(stream.try_next().await, Err(0));
    /// # })
    /// ```
    fn map_ok<T, F>(self, f: F) -> MapOk<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Ok) -> T,
    {
        MapOk::new(self, f)
    }

    /// Wraps the current stream in a new stream which maps the error value
    /// using the provided closure.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let mut stream =
    ///     stream::iter(vec![Ok(5), Err(0)])
    ///         .map_err(|x| x + 2);
    ///
    /// assert_eq!(stream.try_next().await, Ok(Some(5)));
    /// assert_eq!(stream.try_next().await, Err(2));
    /// # })
    /// ```
    fn map_err<E, F>(self, f: F) -> MapErr<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Error) -> E,
    {
        MapErr::new(self, f)
    }

    /// Chain on a computation for when a value is ready, passing the successful
    /// results to the provided closure `f`.
    ///
    /// This function can be used to run a unit of work when the next successful
    /// value on a stream is ready. The closure provided will be yielded a value
    /// when ready, and the returned future will then be run to completion to
    /// produce the next value on this stream.
    ///
    /// Any errors produced by this stream will not be passed to the closure,
    /// and will be passed through.
    ///
    /// The returned value of the closure must implement the `TryFuture` trait
    /// and can represent some more work to be done before the composed stream
    /// is finished.
    ///
    /// Note that this function consumes the receiving stream and returns a
    /// wrapped version of it.
    ///
    /// To process the entire stream and return a single future representing
    /// success or error, use `try_for_each` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::channel::mpsc;
    /// use futures::future;
    /// use futures::stream::TryStreamExt;
    ///
    /// let (_tx, rx) = mpsc::channel::<Result<i32, ()>>(1);
    ///
    /// let rx = rx.and_then(|result| {
    ///     future::ok(if result % 2 == 0 {
    ///         Some(result)
    ///     } else {
    ///         None
    ///     })
    /// });
    /// ```
    fn and_then<Fut, F>(self, f: F) -> AndThen<Self, Fut, F>
        where F: FnMut(Self::Ok) -> Fut,
              Fut: TryFuture<Error = Self::Error>,
              Self: Sized,
    {
        AndThen::new(self, f)
    }

    /// Chain on a computation for when an error happens, passing the
    /// erroneous result to the provided closure `f`.
    ///
    /// This function can be used to run a unit of work and attempt to recover from
    /// an error if one happens. The closure provided will be yielded an error
    /// when one appears, and the returned future will then be run to completion
    /// to produce the next value on this stream.
    ///
    /// Any successful values produced by this stream will not be passed to the
    /// closure, and will be passed through.
    ///
    /// The returned value of the closure must implement the [`TryFuture`](futures_core::future::TryFuture) trait
    /// and can represent some more work to be done before the composed stream
    /// is finished.
    ///
    /// Note that this function consumes the receiving stream and returns a
    /// wrapped version of it.
    fn or_else<Fut, F>(self, f: F) -> OrElse<Self, Fut, F>
        where F: FnMut(Self::Error) -> Fut,
              Fut: TryFuture<Ok = Self::Ok>,
              Self: Sized,
    {
        OrElse::new(self, f)
    }

    /// Do something with the success value of this stream, afterwards passing
    /// it on.
    ///
    /// This is similar to the `StreamExt::inspect` method where it allows
    /// easily inspecting the success value as it passes through the stream, for
    /// example to debug what's going on.
    fn inspect_ok<F>(self, f: F) -> InspectOk<Self, F>
        where F: FnMut(&Self::Ok),
              Self: Sized,
    {
        InspectOk::new(self, f)
    }

    /// Do something with the error value of this stream, afterwards passing it on.
    ///
    /// This is similar to the `StreamExt::inspect` method where it allows
    /// easily inspecting the error value as it passes through the stream, for
    /// example to debug what's going on.
    fn inspect_err<F>(self, f: F) -> InspectErr<Self, F>
        where F: FnMut(&Self::Error),
              Self: Sized,
    {
        InspectErr::new(self, f)
    }

    /// Wraps a [`TryStream`] into a type that implements
    /// [`Stream`](futures_core::Stream)
    ///
    /// [`TryStream`]s currently do not implement the
    /// [`Stream`](futures_core::Stream) trait because of limitations
    /// of the compiler.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::stream::{Stream, TryStream, TryStreamExt};
    ///
    /// # type T = i32;
    /// # type E = ();
    /// fn make_try_stream() -> impl TryStream<Ok = T, Error = E> { // ... }
    /// # futures::stream::empty()
    /// # }
    /// fn take_stream(stream: impl Stream<Item = Result<T, E>>) { /* ... */ }
    ///
    /// take_stream(make_try_stream().into_stream());
    /// ```
    fn into_stream(self) -> IntoStream<Self>
        where Self: Sized,
    {
        IntoStream::new(self)
    }

    /// Creates a future that attempts to resolve the next item in the stream.
    /// If an error is encountered before the next item, the error is returned
    /// instead.
    ///
    /// This is similar to the `Stream::next` combinator, but returns a
    /// `Result<Option<T>, E>` rather than an `Option<Result<T, E>>`, making
    /// for easy use with the `?` operator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let mut stream = stream::iter(vec![Ok(()), Err(())]);
    ///
    /// assert_eq!(stream.try_next().await, Ok(Some(())));
    /// assert_eq!(stream.try_next().await, Err(()));
    /// # })
    /// ```
    fn try_next(&mut self) -> TryNext<'_, Self>
        where Self: Unpin,
    {
        TryNext::new(self)
    }

    /// Attempts to run this stream to completion, executing the provided
    /// asynchronous closure for each element on the stream.
    ///
    /// The provided closure will be called for each item this stream produces,
    /// yielding a future. That future will then be executed to completion
    /// before moving on to the next item.
    ///
    /// The returned value is a [`Future`](futures_core::Future) where the
    /// [`Output`](futures_core::Future::Output) type is
    /// `Result<(), Self::Error>`. If any of the intermediate
    /// futures or the stream returns an error, this future will return
    /// immediately with an error.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let mut x = 0i32;
    ///
    /// {
    ///     let fut = stream::repeat(Ok(1)).try_for_each(|item| {
    ///         x += item;
    ///         future::ready(if x == 3 { Err(()) } else { Ok(()) })
    ///     });
    ///     assert_eq!(fut.await, Err(()));
    /// }
    ///
    /// assert_eq!(x, 3);
    /// # })
    /// ```
    fn try_for_each<Fut, F>(self, f: F) -> TryForEach<Self, Fut, F>
        where F: FnMut(Self::Ok) -> Fut,
              Fut: TryFuture<Ok = (), Error=Self::Error>,
              Self: Sized
    {
        TryForEach::new(self, f)
    }

    /// Skip elements on this stream while the provided asynchronous predicate
    /// resolves to `true`.
    ///
    /// This function is similar to [`StreamExt::skip_while`](crate::stream::StreamExt::skip_while)
    /// but exits early if an error occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let stream = stream::iter(vec![Ok::<i32, i32>(1), Ok(3), Ok(2)]);
    /// let stream = stream.try_skip_while(|x| future::ready(Ok(*x < 3)));
    ///
    /// let output: Result<Vec<i32>, i32> = stream.try_collect().await;
    /// assert_eq!(output, Ok(vec![3, 2]));
    /// # })
    /// ```
    fn try_skip_while<Fut, F>(self, f: F) -> TrySkipWhile<Self, Fut, F>
        where F: FnMut(&Self::Ok) -> Fut,
              Fut: TryFuture<Ok = bool, Error = Self::Error>,
              Self: Sized
    {
        TrySkipWhile::new(self, f)
    }

    /// Attempts to run this stream to completion, executing the provided asynchronous
    /// closure for each element on the stream concurrently as elements become
    /// available, exiting as soon as an error occurs.
    ///
    /// This is similar to
    /// [`StreamExt::for_each_concurrent`](super::StreamExt::for_each_concurrent),
    /// but will resolve to an error immediately if the underlying stream or the provided
    /// closure return an error.
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
    /// use futures::stream::{self, StreamExt, TryStreamExt};
    ///
    /// let (tx1, rx1) = oneshot::channel();
    /// let (tx2, rx2) = oneshot::channel();
    /// let (_tx3, rx3) = oneshot::channel();
    ///
    /// let stream = stream::iter(vec![rx1, rx2, rx3]);
    /// let fut = stream.map(Ok).try_for_each_concurrent(
    ///     /* limit */ 2,
    ///     async move |rx| {
    ///         let res: Result<(), oneshot::Canceled> = rx.await;
    ///         res
    ///     }
    /// );
    ///
    /// tx1.send(()).unwrap();
    /// // Drop the second sender so that `rx2` resolves to `Canceled`.
    /// drop(tx2);
    ///
    /// // The final result is an error because the second future
    /// // resulted in an error.
    /// assert_eq!(Err(oneshot::Canceled), fut.await);
    /// # })
    /// ```
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn try_for_each_concurrent<Fut, F>(
        self,
        limit: impl Into<Option<usize>>,
        f: F,
    ) -> TryForEachConcurrent<Self, Fut, F>
        where F: FnMut(Self::Ok) -> Fut,
              Fut: Future<Output = Result<(), Self::Error>>,
              Self: Sized,
    {
        TryForEachConcurrent::new(self, limit.into(), f)
    }

    /// Attempt to Collect all of the values of this stream into a vector,
    /// returning a future representing the result of that computation.
    ///
    /// This combinator will collect all successful results of this stream and
    /// collect them into a `Vec<Self::Item>`. If an error happens then all
    /// collected elements will be dropped and the error will be returned.
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
    /// use futures::stream::TryStreamExt;
    /// use std::thread;
    ///
    /// let (mut tx, rx) = mpsc::unbounded();
    ///
    /// thread::spawn(move || {
    ///     for i in (1..=5) {
    ///         tx.unbounded_send(Ok(i)).unwrap();
    ///     }
    ///     tx.unbounded_send(Err(6)).unwrap();
    /// });
    ///
    /// let output: Result<Vec<i32>, i32> = rx.try_collect().await;
    /// assert_eq!(output, Err(6));
    /// # })
    /// ```
    fn try_collect<C: Default + Extend<Self::Ok>>(self) -> TryCollect<Self, C>
        where Self: Sized
    {
        TryCollect::new(self)
    }

    /// Attempt to filter the values produced by this stream according to the
    /// provided asynchronous closure.
    ///
    /// As values of this stream are made available, the provided predicate `f`
    /// will be run on them. If the predicate returns a `Future` which resolves
    /// to `true`, then the stream will yield the value, but if the predicate
    /// return a `Future` which resolves to `false`, then the value will be
    /// discarded and the next value will be produced.
    ///
    /// All errors are passed through without filtering in this combinator.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it, similar to the existing `filter` methods in
    /// the standard library.
    ///
    /// # Examples
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, StreamExt, TryStreamExt};
    ///
    /// let stream = stream::iter(vec![Ok(1i32), Ok(2i32), Ok(3i32), Err("error")]);
    /// let mut evens = stream.try_filter(|x| {
    ///     future::ready(x % 2 == 0)
    /// });
    ///
    /// assert_eq!(evens.next().await, Some(Ok(2)));
    /// assert_eq!(evens.next().await, Some(Err("error")));
    /// # })
    /// ```
    fn try_filter<Fut, F>(self, f: F) -> TryFilter<Self, Fut, F>
        where Fut: Future<Output = bool>,
              F: FnMut(&Self::Ok) -> Fut,
              Self: Sized
    {
        TryFilter::new(self, f)
    }

    /// Attempt to filter the values produced by this stream while
    /// simultaneously mapping them to a different type according to the
    /// provided asynchronous closure.
    ///
    /// As values of this stream are made available, the provided function will
    /// be run on them. If the future returned by the predicate `f` resolves to
    /// [`Some(item)`](Some) then the stream will yield the value `item`, but if
    /// it resolves to [`None`] then the next value will be produced.
    ///
    /// All errors are passed through without filtering in this combinator.
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
    /// use futures::stream::{self, StreamExt, TryStreamExt};
    ///
    /// let stream = stream::iter(vec![Ok(1i32), Ok(6i32), Err("error")]);
    /// let mut halves = stream.try_filter_map(|x| {
    ///     let ret = if x % 2 == 0 { Some(x / 2) } else { None };
    ///     future::ready(Ok(ret))
    /// });
    ///
    /// assert_eq!(halves.next().await, Some(Ok(3)));
    /// assert_eq!(halves.next().await, Some(Err("error")));
    /// # })
    /// ```
    fn try_filter_map<Fut, F, T>(self, f: F) -> TryFilterMap<Self, Fut, F>
        where Fut: TryFuture<Ok = Option<T>, Error = Self::Error>,
              F: FnMut(Self::Ok) -> Fut,
              Self: Sized
    {
        TryFilterMap::new(self, f)
    }


    /// Attempt to execute an accumulating asynchronous computation over a
    /// stream, collecting all the values into one final result.
    ///
    /// This combinator will accumulate all values returned by this stream
    /// according to the closure provided. The initial state is also provided to
    /// this method and then is returned again by each execution of the closure.
    /// Once the entire stream has been exhausted the returned future will
    /// resolve to this value.
    ///
    /// This method is similar to [`fold`](super::StreamExt::fold), but will
    /// exit early if an error is encountered in either the stream or the
    /// provided closure.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::stream::{self, TryStreamExt};
    ///
    /// let number_stream = stream::iter(vec![Ok::<i32, i32>(1), Ok(2)]);
    /// let sum = number_stream.try_fold(0, |acc, x| future::ready(Ok(acc + x)));
    /// assert_eq!(sum.await, Ok(3));
    ///
    /// let number_stream_with_err = stream::iter(vec![Ok::<i32, i32>(1), Err(2), Ok(1)]);
    /// let sum = number_stream_with_err.try_fold(0, |acc, x| future::ready(Ok(acc + x)));
    /// assert_eq!(sum.await, Err(2));
    /// # })
    /// ```
    fn try_fold<T, Fut, F>(self, init: T, f: F) -> TryFold<Self, Fut, T, F>
        where F: FnMut(T, Self::Ok) -> Fut,
              Fut: TryFuture<Ok = T, Error = Self::Error>,
              Self: Sized,
    {
        TryFold::new(self, f, init)
    }

    /// Attempt to concatenate all items of a stream into a single
    /// extendable destination, returning a future representing the end result.
    ///
    /// This combinator will extend the first item with the contents of all
    /// the subsequent successful results of the stream. If the stream is empty,
    /// the default value will be returned.
    ///
    /// Works with all collections that implement the [`Extend`](std::iter::Extend) trait.
    ///
    /// This method is similar to [`concat`](super::StreamExt::concat), but will
    /// exit early if an error is encountered in the stream.
    ///
    /// # Examples
    ///
    // TODO: https://github.com/rust-lang-nursery/futures-rs/issues/1657
    /// ```ignore
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::mpsc;
    /// use futures::stream::TryStreamExt;
    /// use std::thread;
    ///
    /// let (mut tx, rx) = mpsc::unbounded::<Result<Vec<i32>, ()>>();
    ///
    /// thread::spawn(move || {
    ///     for i in (0..3).rev() {
    ///         let n = i * 3;
    ///         tx.unbounded_send(Ok(vec![n + 1, n + 2, n + 3])).unwrap();
    ///     }
    /// });
    ///
    /// let result = rx.try_concat().await;
    ///
    /// assert_eq!(result, Ok(vec![7, 8, 9, 4, 5, 6, 1, 2, 3]));
    /// # });
    /// ```
    fn try_concat(self) -> TryConcat<Self>
    where Self: Sized,
          Self::Ok: Extend<<<Self as TryStream>::Ok as IntoIterator>::Item> +
                    IntoIterator + Default,
    {
        TryConcat::new(self)
    }

    /// Attempt to execute several futures from a stream concurrently.
    ///
    /// This stream's `Ok` type must be a [`TryFuture`](futures_core::future::TryFuture) with an `Error` type
    /// that matches the stream's `Error` type.
    ///
    /// This adaptor will buffer up to `n` futures and then return their
    /// outputs in the order in which they complete. If the underlying stream
    /// returns an error, it will be immediately propagated.
    ///
    /// The returned stream will be a stream of results, each containing either
    /// an error or a future's output. An error can be produced either by the
    /// underlying stream itself or by one of the futures it yielded.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// Results are returned in the order of completion:
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::oneshot;
    /// use futures::stream::{self, StreamExt, TryStreamExt};
    ///
    /// let (send_one, recv_one) = oneshot::channel();
    /// let (send_two, recv_two) = oneshot::channel();
    ///
    /// let stream_of_futures = stream::iter(vec![Ok(recv_one), Ok(recv_two)]);
    ///
    /// let mut buffered = stream_of_futures.try_buffer_unordered(10);
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
    ///
    /// Errors from the underlying stream itself are propagated:
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::channel::mpsc;
    /// use futures::future;
    /// use futures::stream::{StreamExt, TryStreamExt};
    ///
    /// let (sink, stream_of_futures) = mpsc::unbounded();
    /// let mut buffered = stream_of_futures.try_buffer_unordered(10);
    ///
    /// sink.unbounded_send(Ok(future::ready(Ok(7i32))))?;
    /// assert_eq!(buffered.next().await, Some(Ok(7i32)));
    ///
    /// sink.unbounded_send(Err("error in the stream"))?;
    /// assert_eq!(buffered.next().await, Some(Err("error in the stream")));
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    #[cfg_attr(
        feature = "cfg-target-has-atomic",
        cfg(all(target_has_atomic = "cas", target_has_atomic = "ptr"))
    )]
    #[cfg(feature = "alloc")]
    fn try_buffer_unordered(self, n: usize) -> TryBufferUnordered<Self>
        where Self::Ok: TryFuture<Error = Self::Error>,
              Self: Sized
    {
        TryBufferUnordered::new(self, n)
    }

    // TODO: false positive warning from rustdoc. Verify once #43466 settles
    //
    /// A convenience method for calling [`TryStream::try_poll_next`] on [`Unpin`]
    /// stream types.
    fn try_poll_next_unpin(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Ok, Self::Error>>>
    where Self: Unpin,
    {
        Pin::new(self).try_poll_next(cx)
    }

    /// Wraps a [`TryStream`] into a stream compatible with libraries using
    /// futures 0.1 `Stream`. Requires the `compat` feature to be enabled.
    /// ```
    /// #![feature(async_await)]
    /// use futures::future::{FutureExt, TryFutureExt};
    /// # let (tx, rx) = futures::channel::oneshot::channel();
    ///
    /// let future03 = async {
    ///     println!("Running on the pool");
    ///     tx.send(42).unwrap();
    /// };
    ///
    /// let future01 = future03
    ///     .unit_error() // Make it a TryFuture
    ///     .boxed()  // Make it Unpin
    ///     .compat();
    ///
    /// tokio::run(future01);
    /// # assert_eq!(42, futures::executor::block_on(rx).unwrap());
    /// ```
    #[cfg(feature = "compat")]
    fn compat(self) -> Compat<Self>
    where
        Self: Sized + Unpin,
    {
        Compat::new(self)
    }


    /// Adapter that converts this stream into an [`AsyncRead`](crate::io::AsyncRead).
    ///
    /// Note that because `into_async_read` moves the stream, the [`Stream`](futures_core::stream::Stream) type must be
    /// [`Unpin`]. If you want to use `into_async_read` with a [`!Unpin`](Unpin) stream, you'll
    /// first have to pin the stream. This can be done by boxing the stream using [`Box::pin`]
    /// or pinning it to the stack using the `pin_mut!` macro from the `pin_utils` crate.
    ///
    /// This method is only available when the `std` feature of this
    /// library is activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, TryStreamExt};
    /// use futures::io::AsyncReadExt;
    ///
    /// let stream = stream::iter(vec![Ok(vec![1, 2, 3, 4, 5])]);
    /// let mut reader = stream.into_async_read();
    /// let mut buf = Vec::new();
    ///
    /// assert!(reader.read_to_end(&mut buf).await.is_ok());
    /// assert_eq!(buf, &[1, 2, 3, 4, 5]);
    /// # })
    /// ```
    #[cfg(feature = "std")]
    fn into_async_read(self) -> IntoAsyncRead<Self>
    where
        Self: Sized + TryStreamExt<Error = std::io::Error> + Unpin,
        Self::Ok: AsRef<[u8]>,
    {
        IntoAsyncRead::new(self)
    }
}
