use crate::{Poll, Sink};
use futures_channel::mpsc::{SendError, Sender, TrySendError, UnboundedSender};
use futures_core::task::Context;
use std::pin::Pin;

impl<T> Sink<T> for Sender<T> {
    type SinkError = SendError;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        (*self).poll_ready(cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        msg: T,
    ) -> Result<(), Self::SinkError> {
        (*self).start_send(msg)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        match (*self).poll_ready(cx) {
            Poll::Ready(Err(ref e)) if e.is_disconnected() => {
                // If the receiver disconnected, we consider the sink to be flushed.
                Poll::Ready(Ok(()))
            }
            x => x,
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        self.disconnect();
        Poll::Ready(Ok(()))
    }
}

impl<T> Sink<T> for UnboundedSender<T> {
    type SinkError = SendError;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        UnboundedSender::poll_ready(&*self, cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        msg: T,
    ) -> Result<(), Self::SinkError> {
        UnboundedSender::start_send(&mut *self, msg)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        self.disconnect();
        Poll::Ready(Ok(()))
    }
}

impl<T> Sink<T> for &UnboundedSender<T> {
    type SinkError = SendError;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        UnboundedSender::poll_ready(*self, cx)
    }

    fn start_send(self: Pin<&mut Self>, msg: T) -> Result<(), Self::SinkError> {
        self.unbounded_send(msg)
            .map_err(TrySendError::into_send_error)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::SinkError>> {
        self.close_channel();
        Poll::Ready(Ok(()))
    }
}
