use std::time::Duration;

use actix_rt::time::sleep;
use futures::{future::ready, stream::repeat, stream::Stream, FutureExt, StreamExt};

/// Interval Stream
pub fn interval_stream(timeout: Duration) -> impl Stream<Item = ()> {
    repeat(()).then(move |_| sleep(timeout).then(|_| ready(())))
}
