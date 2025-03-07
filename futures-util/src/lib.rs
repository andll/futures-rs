//! Combinators and utilities for working with `Future`s, `Stream`s, `Sink`s,
//! and the `AsyncRead` and `AsyncWrite` traits.

#![cfg_attr(feature = "async-await", feature(async_await))]
#![cfg_attr(feature = "cfg-target-has-atomic", feature(cfg_target_has_atomic))]
#![cfg_attr(feature = "never-type", feature(never_type))]

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
// It cannot be included in the published code because this lints have false positives in the minimum required version.
#![cfg_attr(test, warn(single_use_lifetimes))]
#![warn(clippy::all)]

#![doc(test(attr(deny(warnings), allow(dead_code, unused_assignments, unused_variables))))]

#![doc(html_root_url = "https://rust-lang-nursery.github.io/futures-api-docs/0.3.0-alpha.16/futures_util")]

#[cfg(all(feature = "cfg-target-has-atomic", not(feature = "nightly")))]
compile_error!("The `cfg-target-has-atomic` feature requires the `nightly` feature as an explicit opt-in to unstable features");

#[cfg(all(feature = "never-type", not(feature = "nightly")))]
compile_error!("The `never-type` feature requires the `nightly` feature as an explicit opt-in to unstable features");

#[cfg(all(feature = "async-await", not(feature = "nightly")))]
compile_error!("The `async-await` feature requires the `nightly` feature as an explicit opt-in to unstable features");

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod macros;

// Re-export pin_mut! for convenience
pub use pin_utils::pin_mut;

#[cfg(feature = "async-await")]
#[macro_use]
#[doc(hidden)]
pub mod async_await;
#[cfg(feature = "async-await")]
#[doc(hidden)]
pub use self::async_await::*;

#[cfg(feature = "async-await")]
#[doc(hidden)]
pub mod rand_reexport { // used by select!
    #[doc(hidden)]
    pub use rand::{prelude::SliceRandom, thread_rng};
}

#[doc(hidden)]
pub mod core_reexport {
    #[doc(hidden)]
    pub use core::*;
}

macro_rules! delegate_sink {
    ($field:ident, $item:ty) => {
        fn poll_ready(
            self: Pin<&mut Self>,
            cx: &mut $crate::core_reexport::task::Context<'_>,
        ) -> $crate::core_reexport::task::Poll<Result<(), Self::SinkError>> {
            self.$field().poll_ready(cx)
        }

        fn start_send(
            self: Pin<&mut Self>,
            item: $item,
        ) -> Result<(), Self::SinkError> {
            self.$field().start_send(item)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut $crate::core_reexport::task::Context<'_>,
        ) -> $crate::core_reexport::task::Poll<Result<(), Self::SinkError>> {
            self.$field().poll_flush(cx)
        }

        fn poll_close(
            self: Pin<&mut Self>,
            cx: &mut $crate::core_reexport::task::Context<'_>,
        ) -> $crate::core_reexport::task::Poll<Result<(), Self::SinkError>> {
            self.$field().poll_close(cx)
        }
    }
}

pub mod future;
#[doc(hidden)] pub use crate::future::FutureExt;

pub mod try_future;
#[doc(hidden)] pub use crate::try_future::TryFutureExt;

pub mod stream;
#[doc(hidden)] pub use crate::stream::StreamExt;

pub mod try_stream;
#[doc(hidden)] pub use crate::try_stream::TryStreamExt;

pub mod sink;
#[doc(hidden)] pub use crate::sink::SinkExt;

pub mod task;

#[cfg(feature = "compat")]
pub mod compat;

#[cfg(feature = "std")]
pub mod io;
#[cfg(feature = "std")]
#[doc(hidden)] pub use crate::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt, AsyncBufReadExt};

cfg_target_has_atomic! {
    #[cfg(feature = "alloc")]
    pub mod lock;
}
