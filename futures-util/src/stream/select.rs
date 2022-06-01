use super::assert_stream;
use crate::stream::{select_with_strategy, PollNext, SelectWithStrategy, ClosedStreams, ExitStrategy};
use core::pin::Pin;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`select()`] function.
    #[derive(Debug)]
    #[must_use = "streams do nothing unless polled"]
    pub struct Select<St1, St2, Exit> {
        #[pin]
        inner: SelectWithStrategy<St1, St2, fn(&mut PollNext)-> PollNext, PollNext, Exit>,
    }
}

#[derive(Debug)]
pub struct ExitWhenBothFinished {}

impl ExitStrategy for ExitWhenBothFinished {
    fn is_terminated(closed_streams: ClosedStreams) -> bool {
        match closed_streams {
            ClosedStreams::Both => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct ExitWhenEitherFinished {}

impl ExitStrategy for ExitWhenEitherFinished {
    fn is_terminated(closed_streams: ClosedStreams) -> bool {
        match closed_streams {
            ClosedStreams::None => false,
            _ => true,
        }
    }
}


fn round_robin(last: &mut PollNext) -> PollNext {
    last.toggle()
}

/// This function will attempt to pull items from both streams. Each
/// stream will be polled in a round-robin fashion, and whenever a stream is
/// ready to yield an item that item is yielded.
///
/// After one of the two input streams completes, the remaining one will be
/// polled exclusively. The returned stream completes when both input
/// streams have completed.
///
/// Note that this function consumes both streams and returns a wrapped
/// version of them.
///
/// ## Examples
///
/// ```rust
/// # futures::executor::block_on(async {
/// use futures::stream::{ repeat, select, StreamExt };
///
/// let left = repeat(1);
/// let right = repeat(2);
///
/// let mut out = select(left, right);
///
/// for _ in 0..100 {
///     // We should be alternating.
///     assert_eq!(1, out.select_next_some().await);
///     assert_eq!(2, out.select_next_some().await);
/// }
/// # });
/// ```
pub fn select<St1, St2>(stream1: St1, stream2: St2) -> Select<St1, St2, ExitWhenBothFinished>
where
    St1: Stream,
    St2: Stream<Item = St1::Item>,
{
    assert_stream::<St1::Item, _>(Select {
        inner: select_with_strategy(stream1, stream2, round_robin),
    })
}


/// Same as `select`, but finishes when either stream finishes
pub fn select_early_exit<St1, St2>(stream1: St1, stream2: St2) -> Select<St1, St2, ExitWhenEitherFinished>
where
    St1: Stream,
    St2: Stream<Item = St1::Item>,
{
    assert_stream::<St1::Item, _>(Select {
        inner: select_with_strategy(stream1, stream2, round_robin),
    })
}

impl<St1, St2, Exit> Select<St1, St2, Exit> {
    /// Acquires a reference to the underlying streams that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> (&St1, &St2) {
        self.inner.get_ref()
    }

    /// Acquires a mutable reference to the underlying streams that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> (&mut St1, &mut St2) {
        self.inner.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying streams that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> (Pin<&mut St1>, Pin<&mut St2>) {
        let this = self.project();
        this.inner.get_pin_mut()
    }

    /// Consumes this combinator, returning the underlying streams.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> (St1, St2) {
        self.inner.into_inner()
    }
}

impl<St1, St2, Exit> FusedStream for Select<St1, St2, Exit>
where
    St1: Stream,
    St2: Stream<Item = St1::Item>,
    Exit: ExitStrategy,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

impl<St1, St2, Exit> Stream for Select<St1, St2, Exit>
where
    St1: Stream,
    St2: Stream<Item = St1::Item>,
    Exit: ExitStrategy,
{
    type Item = St1::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<St1::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }
}
