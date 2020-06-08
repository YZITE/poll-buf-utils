use std::pin::Pin;
use std::task::{Context, Poll};

pub fn poll_write<I, O>(
    input: &mut I,
    mut output: Pin<&mut O>,
    cx: &mut Context<'_>,
) -> Poll<std::io::Result<()>>
where
    I: bytes::Buf,
    O: futures_io::AsyncWrite,
{
    while input.has_remaining() {
        match output.as_mut().poll_write(cx, input.bytes()) {
            // if we managed to write something...
            Poll::Ready(Ok(n)) if n != 0 => input.advance(n),

            // assumption: if we get here, the call to poll_write failed and
            // didn't write anything
            ret => return ret.map(|x| x.map(|_n: usize| ())),
        }
    }
    Poll::Ready(Ok(()))
}
