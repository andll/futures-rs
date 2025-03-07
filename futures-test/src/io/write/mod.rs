//! Additional combinators for testing async writers.

use futures_io::AsyncWrite;

pub use super::limited::Limited;
pub use crate::interleave_pending::InterleavePending;

/// Additional combinators for testing async writers.
pub trait AsyncWriteTestExt: AsyncWrite {
    /// Introduces an extra [`Poll::Pending`](futures_core::task::Poll::Pending)
    /// in between each operation on the writer.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// use futures::task::Poll;
    /// use futures::io::AsyncWrite;
    /// use futures_test::task::noop_context;
    /// use futures_test::io::AsyncWriteTestExt;
    /// use pin_utils::pin_mut;
    ///
    /// let writer = std::io::Cursor::new([0u8; 4]).interleave_pending_write();
    /// pin_mut!(writer);
    ///
    /// let mut cx = noop_context();
    ///
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[1, 2])?, Poll::Pending);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[1, 2])?, Poll::Ready(2));
    /// assert_eq!(writer.get_ref().get_ref(), &[1, 2, 0, 0]);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[3, 4])?, Poll::Pending);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[3, 4])?, Poll::Ready(2));
    /// assert_eq!(writer.get_ref().get_ref(), &[1, 2, 3, 4]);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[5, 6])?, Poll::Pending);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[5, 6])?, Poll::Ready(0));
    ///
    /// assert_eq!(writer.as_mut().poll_flush(&mut cx)?, Poll::Pending);
    /// assert_eq!(writer.as_mut().poll_flush(&mut cx)?, Poll::Ready(()));
    ///
    /// assert_eq!(writer.as_mut().poll_close(&mut cx)?, Poll::Pending);
    /// assert_eq!(writer.as_mut().poll_close(&mut cx)?, Poll::Ready(()));
    ///
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn interleave_pending_write(self) -> InterleavePending<Self>
    where
        Self: Sized,
    {
        InterleavePending::new(self)
    }

    /// Limit the number of bytes allowed to be written on each call to `poll_write`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// use futures::task::Poll;
    /// use futures::io::AsyncWrite;
    /// use futures_test::task::noop_context;
    /// use futures_test::io::AsyncWriteTestExt;
    /// use pin_utils::pin_mut;
    ///
    /// let writer = std::io::Cursor::new([0u8; 4]).limited_write(2);
    /// pin_mut!(writer);
    ///
    /// let mut cx = noop_context();
    ///
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[1, 2])?, Poll::Ready(2));
    /// assert_eq!(writer.get_ref().get_ref(), &[1, 2, 0, 0]);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[3])?, Poll::Ready(1));
    /// assert_eq!(writer.get_ref().get_ref(), &[1, 2, 3, 0]);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[4, 5])?, Poll::Ready(1));
    /// assert_eq!(writer.get_ref().get_ref(), &[1, 2, 3, 4]);
    /// assert_eq!(writer.as_mut().poll_write(&mut cx, &[5])?, Poll::Ready(0));
    ///
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn limited_write(self, limit: usize) -> Limited<Self>
    where
        Self: Sized,
    {
        Limited::new(self, limit)
    }
}

impl<W> AsyncWriteTestExt for W where W: AsyncWrite {}
