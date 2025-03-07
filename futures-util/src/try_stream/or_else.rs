use core::fmt;
use core::pin::Pin;
use futures_core::future::TryFuture;
use futures_core::stream::{Stream, TryStream};
use futures_core::task::{Context, Poll};
use futures_sink::Sink;
use pin_utils::{unsafe_pinned, unsafe_unpinned};

/// Stream for the [`or_else`](super::TryStreamExt::or_else) method.
#[must_use = "streams do nothing unless polled"]
pub struct OrElse<St, Fut, F> {
    stream: St,
    future: Option<Fut>,
    f: F,
}

impl<St: Unpin, Fut: Unpin, F> Unpin for OrElse<St, Fut, F> {}

impl<St, Fut, F> fmt::Debug for OrElse<St, Fut, F>
where
    St: fmt::Debug,
    Fut: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrElse")
            .field("stream", &self.stream)
            .field("future", &self.future)
            .finish()
    }
}

impl<St, Fut, F> OrElse<St, Fut, F>
    where St: TryStream,
          F: FnMut(St::Error) -> Fut,
          Fut: TryFuture<Ok = St::Ok>,
{
    unsafe_pinned!(stream: St);
    unsafe_pinned!(future: Option<Fut>);
    unsafe_unpinned!(f: F);

    pub(super) fn new(stream: St, f: F) -> Self {
        Self { stream, future: None, f }
    }

    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &St {
        &self.stream
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> &mut St {
        &mut self.stream
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_pin_mut<'a>(self: Pin<&'a mut Self>) -> Pin<&'a mut St> {
        self.stream()
    }

    /// Consumes this combinator, returning the underlying stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> St {
        self.stream
    }
}

impl<St, Fut, F> Stream for OrElse<St, Fut, F>
    where St: TryStream,
          F: FnMut(St::Error) -> Fut,
          Fut: TryFuture<Ok = St::Ok>,
{
    type Item = Result<St::Ok, Fut::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.future.is_none() {
            let item = match ready!(self.as_mut().stream().try_poll_next(cx)) {
                None => return Poll::Ready(None),
                Some(Ok(e)) => return Poll::Ready(Some(Ok(e))),
                Some(Err(e)) => e,
            };
            let fut = (self.as_mut().f())(item);
            self.as_mut().future().set(Some(fut));
        }

        let e = ready!(self.as_mut().future().as_pin_mut().unwrap().try_poll(cx));
        self.as_mut().future().set(None);
        Poll::Ready(Some(e))
    }
}

// Forwarding impl of Sink from the underlying stream
impl<S, Fut, F, Item> Sink<Item> for OrElse<S, Fut, F>
    where S: TryStream + Sink<Item>,
          F: FnMut(S::Error) -> Fut,
          Fut: TryFuture<Ok = S::Ok>,
{
    type SinkError = S::SinkError;

    delegate_sink!(stream, Item);
}
