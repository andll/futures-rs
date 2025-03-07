//! IO
//!
//! This module contains a number of functions for working with
//! `AsyncRead`, `AsyncWrite`, `AsyncSeek`, and `AsyncBufRead` types, including
//! the `AsyncReadExt`, `AsyncWriteExt`, `AsyncSeekExt`, and `AsyncBufReadExt`
//! traits which add methods to the `AsyncRead`, `AsyncWrite`, `AsyncSeek`,
//! and `AsyncBufRead` types.
//!
//! This module is only available when the `std` feature of this
//! library is activated, and it is activated by default.

pub use futures_io::{
    AsyncRead, AsyncWrite, AsyncSeek, AsyncBufRead, IoSlice, IoSliceMut, SeekFrom,
};

#[cfg(feature = "io-compat")] use crate::compat::Compat;

// used by `BufReader` and `BufWriter`
// https://github.com/rust-lang/rust/blob/master/src/libstd/sys_common/io.rs#L1
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

mod allow_std;
pub use self::allow_std::AllowStdIo;

mod buf_reader;
pub use self::buf_reader::BufReader;

mod buf_writer;
pub use self::buf_writer::BufWriter;

mod copy_into;
pub use self::copy_into::CopyInto;

mod copy_buf_into;
pub use self::copy_buf_into::CopyBufInto;

mod flush;
pub use self::flush::Flush;

mod into_sink;
pub use self::into_sink::IntoSink;

mod lines;
pub use self::lines::Lines;

mod read;
pub use self::read::Read;

mod read_vectored;
pub use self::read_vectored::ReadVectored;

mod read_exact;
pub use self::read_exact::ReadExact;

mod read_line;
pub use self::read_line::ReadLine;

mod read_to_end;
pub use self::read_to_end::ReadToEnd;

mod read_until;
pub use self::read_until::ReadUntil;

mod close;
pub use self::close::Close;

mod seek;
pub use self::seek::Seek;

mod split;
pub use self::split::{ReadHalf, WriteHalf};

mod window;
pub use self::window::Window;

mod write;
pub use self::write::Write;

mod write_vectored;
pub use self::write_vectored::WriteVectored;

mod write_all;
pub use self::write_all::WriteAll;

/// An extension trait which adds utility methods to `AsyncRead` types.
pub trait AsyncReadExt: AsyncRead {
    /// Creates a future which copies all the bytes from one object to another.
    ///
    /// The returned future will copy all the bytes read from this `AsyncRead` into the
    /// `writer` specified. This future will only complete once the `reader` has hit
    /// EOF and all bytes have been written to and flushed from the `writer`
    /// provided.
    ///
    /// On success the number of bytes is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::{AsyncReadExt, AsyncWriteExt};
    /// use std::io::Cursor;
    ///
    /// let reader = Cursor::new([1, 2, 3, 4]);
    /// let mut writer = Cursor::new([0u8; 5]);
    ///
    /// let bytes = reader.copy_into(&mut writer).await?;
    /// writer.close().await?;
    ///
    /// assert_eq!(bytes, 4);
    /// assert_eq!(writer.into_inner(), [1, 2, 3, 4, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn copy_into<W>(self, writer: &mut W) -> CopyInto<'_, Self, W>
    where
        Self: Sized,
        W: AsyncWrite + Unpin + ?Sized,
    {
        CopyInto::new(self, writer)
    }

    /// Tries to read some bytes directly into the given `buf` in asynchronous
    /// manner, returning a future type.
    ///
    /// The returned future will resolve to the number of bytes read once the read
    /// operation is completed.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncReadExt;
    /// use std::io::Cursor;
    ///
    /// let mut reader = Cursor::new([1, 2, 3, 4]);
    /// let mut output = [0u8; 5];
    ///
    /// let bytes = reader.read(&mut output[..]).await?;
    ///
    /// // This is only guaranteed to be 4 because `&[u8]` is a synchronous
    /// // reader. In a real system you could get anywhere from 1 to
    /// // `output.len()` bytes in a single read.
    /// assert_eq!(bytes, 4);
    /// assert_eq!(output, [1, 2, 3, 4, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>
        where Self: Unpin,
    {
        Read::new(self, buf)
    }

    /// Creates a future which will read from the `AsyncRead` into `bufs` using vectored
    /// IO operations.
    ///
    /// The returned future will resolve to the number of bytes read once the read
    /// operation is completed.
    fn read_vectored<'a>(&'a mut self, bufs: &'a mut [IoSliceMut<'a>]) -> ReadVectored<'a, Self>
        where Self: Unpin,
    {
        ReadVectored::new(self, bufs)
    }

    /// Creates a future which will read exactly enough bytes to fill `buf`,
    /// returning an error if end of file (EOF) is hit sooner.
    ///
    /// The returned future will resolve once the read operation is completed.
    ///
    /// In the case of an error the buffer and the object will be discarded, with
    /// the error yielded.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncReadExt;
    /// use std::io::Cursor;
    ///
    /// let mut reader = Cursor::new([1, 2, 3, 4]);
    /// let mut output = [0u8; 4];
    ///
    /// reader.read_exact(&mut output).await?;
    ///
    /// assert_eq!(output, [1, 2, 3, 4]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    ///
    /// ## EOF is hit before `buf` is filled
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncReadExt;
    /// use std::io::{self, Cursor};
    ///
    /// let mut reader = Cursor::new([1, 2, 3, 4]);
    /// let mut output = [0u8; 5];
    ///
    /// let result = reader.read_exact(&mut output).await;
    ///
    /// assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    /// # });
    /// ```
    fn read_exact<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> ReadExact<'a, Self>
        where Self: Unpin,
    {
        ReadExact::new(self, buf)
    }

    /// Creates a future which will read all the bytes from this `AsyncRead`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncReadExt;
    /// use std::io::Cursor;
    ///
    /// let mut reader = Cursor::new([1, 2, 3, 4]);
    /// let mut output = Vec::with_capacity(4);
    ///
    /// reader.read_to_end(&mut output).await?;
    ///
    /// assert_eq!(output, vec![1, 2, 3, 4]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> ReadToEnd<'a, Self>
        where Self: Unpin,
    {
        ReadToEnd::new(self, buf)
    }

    /// Helper method for splitting this read/write object into two halves.
    ///
    /// The two halves returned implement the `AsyncRead` and `AsyncWrite`
    /// traits, respectively.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncReadExt;
    /// use std::io::Cursor;
    ///
    /// // Note that for `Cursor` the read and write halves share a single
    /// // seek position. This may or may not be true for other types that
    /// // implement both `AsyncRead` and `AsyncWrite`.
    ///
    /// let reader = Cursor::new([1, 2, 3, 4]);
    /// let mut buffer = Cursor::new([0, 0, 0, 0, 5, 6, 7, 8]);
    /// let mut writer = Cursor::new([0u8; 5]);
    ///
    /// {
    ///     let (buffer_reader, mut buffer_writer) = (&mut buffer).split();
    ///     reader.copy_into(&mut buffer_writer).await?;
    ///     buffer_reader.copy_into(&mut writer).await?;
    /// }
    ///
    /// assert_eq!(buffer.into_inner(), [1, 2, 3, 4, 5, 6, 7, 8]);
    /// assert_eq!(writer.into_inner(), [5, 6, 7, 8, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn split(self) -> (ReadHalf<Self>, WriteHalf<Self>)
        where Self: AsyncWrite + Sized,
    {
        split::split(self)
    }

    /// Wraps an [`AsyncRead`] in a compatibility wrapper that allows it to be
    /// used as a futures 0.1 / tokio-io 0.1 `AsyncRead`. If the wrapped type
    /// implements [`AsyncWrite`] as well, the result will also implement the
    /// futures 0.1 / tokio 0.1 `AsyncWrite` trait.
    ///
    /// Requires the `io-compat` feature to enable.
    #[cfg(feature = "io-compat")]
    fn compat(self) -> Compat<Self>
        where Self: Sized + Unpin,
    {
        Compat::new(self)
    }
}

impl<R: AsyncRead + ?Sized> AsyncReadExt for R {}

/// An extension trait which adds utility methods to `AsyncWrite` types.
pub trait AsyncWriteExt: AsyncWrite {
    /// Creates a future which will entirely flush this `AsyncWrite`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::{AllowStdIo, AsyncWriteExt};
    /// use std::io::{BufWriter, Cursor};
    ///
    /// let mut output = [0u8; 5];
    ///
    /// {
    ///     let writer = Cursor::new(&mut output[..]);
    ///     let mut buffered = AllowStdIo::new(BufWriter::new(writer));
    ///     buffered.write_all(&[1, 2]).await?;
    ///     buffered.write_all(&[3, 4]).await?;
    ///     buffered.flush().await?;
    /// }
    ///
    /// assert_eq!(output, [1, 2, 3, 4, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn flush(&mut self) -> Flush<'_, Self>
        where Self: Unpin,
    {
        Flush::new(self)
    }

    /// Creates a future which will entirely close this `AsyncWrite`.
    fn close(&mut self) -> Close<'_, Self>
        where Self: Unpin,
    {
        Close::new(self)
    }

    /// Creates a future which will write bytes from `buf` into the object.
    ///
    /// The returned future will resolve to the number of bytes written once the write
    /// operation is completed.
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self>
        where Self: Unpin,
    {
        Write::new(self, buf)
    }

    /// Creates a future which will write bytes from `bufs` into the object using vectored
    /// IO operations.
    ///
    /// The returned future will resolve to the number of bytes written once the write
    /// operation is completed.
    fn write_vectored<'a>(&'a mut self, bufs: &'a [IoSlice<'a>]) -> WriteVectored<'a, Self>
        where Self: Unpin,
    {
        WriteVectored::new(self, bufs)
    }

    /// Write data into this object.
    ///
    /// Creates a future that will write the entire contents of the buffer `buf` into
    /// this `AsyncWrite`.
    ///
    /// The returned future will not complete until all the data has been written.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncWriteExt;
    /// use std::io::Cursor;
    ///
    /// let mut writer = Cursor::new([0u8; 5]);
    ///
    /// writer.write_all(&[1, 2, 3, 4]).await?;
    ///
    /// assert_eq!(writer.into_inner(), [1, 2, 3, 4, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>
        where Self: Unpin,
    {
        WriteAll::new(self, buf)
    }

    /// Wraps an [`AsyncWrite`] in a compatibility wrapper that allows it to be
    /// used as a futures 0.1 / tokio-io 0.1 `AsyncWrite`.
    /// Requires the `io-compat` feature to enable.
    #[cfg(feature = "io-compat")]
    fn compat_write(self) -> Compat<Self>
        where Self: Sized + Unpin,
    {
        Compat::new(self)
    }


    /// Allow using an [`AsyncWrite`] as a [`Sink`](futures_sink::Sink)`<Item: AsRef<[u8]>>`.
    ///
    /// This adapter produces a sink that will write each value passed to it
    /// into the underlying writer.
    ///
    /// Note that this function consumes the given writer, returning a wrapped
    /// version.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncWriteExt;
    /// use futures::stream::{self, StreamExt};
    ///
    /// let stream = stream::iter(vec![Ok([1, 2, 3]), Ok([4, 5, 6])]);
    ///
    /// let mut writer = vec![];
    ///
    /// stream.forward((&mut writer).into_sink()).await?;
    ///
    /// assert_eq!(writer, vec![1, 2, 3, 4, 5, 6]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn into_sink<Item: AsRef<[u8]>>(self) -> IntoSink<Self, Item>
        where Self: Sized,
    {
        IntoSink::new(self)
    }
}

impl<W: AsyncWrite + ?Sized> AsyncWriteExt for W {}

/// An extension trait which adds utility methods to `AsyncSeek` types.
pub trait AsyncSeekExt: AsyncSeek {
    /// Creates a future which will seek an IO object, and then yield the
    /// new position in the object and the object itself.
    ///
    /// In the case of an error the buffer and the object will be discarded, with
    /// the error yielded.
    fn seek(&mut self, pos: SeekFrom) -> Seek<'_, Self>
        where Self: Unpin,
    {
        Seek::new(self, pos)
    }
}

impl<S: AsyncSeek + ?Sized> AsyncSeekExt for S {}

/// An extension trait which adds utility methods to `AsyncBufRead` types.
pub trait AsyncBufReadExt: AsyncBufRead {
    /// Creates a future which copies all the bytes from one object to another.
    ///
    /// The returned future will copy all the bytes read from this `AsyncBufRead` into the
    /// `writer` specified. This future will only complete once the `reader` has hit
    /// EOF and all bytes have been written to and flushed from the `writer`
    /// provided.
    ///
    /// On success the number of bytes is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::{AsyncBufReadExt, AsyncWriteExt};
    /// use std::io::Cursor;
    ///
    /// let reader = Cursor::new([1, 2, 3, 4]);
    /// let mut writer = Cursor::new([0u8; 5]);
    ///
    /// let bytes = reader.copy_buf_into(&mut writer).await?;
    /// writer.close().await?;
    ///
    /// assert_eq!(bytes, 4);
    /// assert_eq!(writer.into_inner(), [1, 2, 3, 4, 0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn copy_buf_into<W>(self, writer: &mut W) -> CopyBufInto<'_, Self, W>
    where
        Self: Sized,
        W: AsyncWrite + Unpin + ?Sized,
    {
        CopyBufInto::new(self, writer)
    }

    /// Creates a future which will read all the bytes associated with this I/O
    /// object into `buf` until the delimiter `byte` or EOF is reached.
    /// This method is the async equivalent to [`BufRead::read_until`](std::io::BufRead::read_until).
    ///
    /// This function will read bytes from the underlying stream until the
    /// delimiter or EOF is found. Once found, all bytes up to, and including,
    /// the delimiter (if found) will be appended to `buf`.
    ///
    /// The returned future will resolve to the number of bytes read once the read
    /// operation is completed.
    ///
    /// In the case of an error the buffer and the object will be discarded, with
    /// the error yielded.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncBufReadExt;
    /// use std::io::Cursor;
    ///
    /// let mut cursor = Cursor::new(b"lorem-ipsum");
    /// let mut buf = vec![];
    ///
    /// // cursor is at 'l'
    /// let num_bytes = cursor.read_until(b'-', &mut buf).await?;
    /// assert_eq!(num_bytes, 6);
    /// assert_eq!(buf, b"lorem-");
    /// buf.clear();
    ///
    /// // cursor is at 'i'
    /// let num_bytes = cursor.read_until(b'-', &mut buf).await?;
    /// assert_eq!(num_bytes, 5);
    /// assert_eq!(buf, b"ipsum");
    /// buf.clear();
    ///
    /// // cursor is at EOF
    /// let num_bytes = cursor.read_until(b'-', &mut buf).await?;
    /// assert_eq!(num_bytes, 0);
    /// assert_eq!(buf, b"");
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn read_until<'a>(
        &'a mut self,
        byte: u8,
        buf: &'a mut Vec<u8>,
    ) -> ReadUntil<'a, Self>
        where Self: Unpin,
    {
        ReadUntil::new(self, byte, buf)
    }

    /// Creates a future which will read all the bytes associated with this I/O
    /// object into `buf` until a newline (the 0xA byte) or EOF is reached,
    /// This method is the async equivalent to [`BufRead::read_line`](std::io::BufRead::read_line).
    ///
    /// This function will read bytes from the underlying stream until the
    /// newline delimiter (the 0xA byte) or EOF is found. Once found, all bytes
    /// up to, and including, the delimiter (if found) will be appended to
    /// `buf`.
    ///
    /// The returned future will resolve to the number of bytes read once the read
    /// operation is completed.
    ///
    /// In the case of an error the buffer and the object will be discarded, with
    /// the error yielded.
    ///
    /// # Errors
    ///
    /// This function has the same error semantics as [`read_until`] and will
    /// also return an error if the read bytes are not valid UTF-8. If an I/O
    /// error is encountered then `buf` may contain some bytes already read in
    /// the event that all data read so far was valid UTF-8.
    ///
    /// [`read_until`]: AsyncBufReadExt::read_until
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncBufReadExt;
    /// use std::io::Cursor;
    ///
    /// let mut cursor = Cursor::new(b"foo\nbar");
    /// let mut buf = String::new();
    ///
    /// // cursor is at 'f'
    /// let num_bytes = cursor.read_line(&mut buf).await?;
    /// assert_eq!(num_bytes, 4);
    /// assert_eq!(buf, "foo\n");
    /// buf.clear();
    ///
    /// // cursor is at 'b'
    /// let num_bytes = cursor.read_line(&mut buf).await?;
    /// assert_eq!(num_bytes, 3);
    /// assert_eq!(buf, "bar");
    /// buf.clear();
    ///
    /// // cursor is at EOF
    /// let num_bytes = cursor.read_line(&mut buf).await?;
    /// assert_eq!(num_bytes, 0);
    /// assert_eq!(buf, "");
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn read_line<'a>(&'a mut self, buf: &'a mut String) -> ReadLine<'a, Self>
        where Self: Unpin,
    {
        ReadLine::new(self, buf)
    }

    /// Returns a stream over the lines of this reader.
    /// This method is the async equivalent to [`BufRead::lines`](std::io::BufRead::lines).
    ///
    /// The stream returned from this function will yield instances of
    /// [`io::Result`]`<`[`String`]`>`. Each string returned will *not* have a newline
    /// byte (the 0xA byte) or CRLF (0xD, 0xA bytes) at the end.
    ///
    /// [`io::Result`]: std::io::Result
    /// [`String`]: String
    ///
    /// # Errors
    ///
    /// Each line of the stream has the same error semantics as [`AsyncBufReadExt::read_line`].
    ///
    /// [`AsyncBufReadExt::read_line`]: AsyncBufReadExt::read_line
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    /// # futures::executor::block_on(async {
    /// use futures::io::AsyncBufReadExt;
    /// use futures::stream::StreamExt;
    /// use std::io::Cursor;
    ///
    /// let cursor = Cursor::new(b"lorem\nipsum\r\ndolor");
    ///
    /// let mut lines_stream = cursor.lines().map(|l| l.unwrap());
    /// assert_eq!(lines_stream.next().await, Some(String::from("lorem")));
    /// assert_eq!(lines_stream.next().await, Some(String::from("ipsum")));
    /// assert_eq!(lines_stream.next().await, Some(String::from("dolor")));
    /// assert_eq!(lines_stream.next().await, None);
    /// # Ok::<(), Box<dyn std::error::Error>>(()) }).unwrap();
    /// ```
    fn lines(self) -> Lines<Self>
        where Self: Sized,
    {
        Lines::new(self)
    }
}

impl<R: AsyncBufRead + ?Sized> AsyncBufReadExt for R {}
