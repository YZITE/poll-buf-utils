/*!
This is an utility library for resumable byte transfers between buffers
supported by the [`bytes`] crate and byte-streams which support the
[`futures_io`] [`AsyncRead`](futures_io::AsyncRead) and/or
[`AsyncWrite`](futures_io::AsyncWrite) traits.

This crate assumes the following behavoirs about `AsyncRead/AsyncWrite` implementations: If the `poll_*` method call results in:
 * `Poll::Ready(Ok(n))` with `n != 0`, bytes were successfully transferred
 * otherwise, we assume that the call failed and no bytes were transferred at all

 **/

use std::pin::Pin;
use std::task::{Context, Poll};

fn ret_reduce(
    ret: Poll<std::io::Result<usize>>,
    reached_limit: bool,
) -> Poll<std::io::Result<bool>> {
    ret.map(|x| x.map(|_| reached_limit))
}

#[derive(Debug)]
pub struct PollResult {
    /// how much bytes were successfully transferred until yielding
    pub delta: usize,

    /// yielded with the following result
    /// the inner `bool` specifies if some pre-specified limit was reached
    pub ret: Poll<std::io::Result<bool>>,
}

/// This function tries to read at most `delta_limit` bytes from `input` to `output`.
pub fn poll_read<I, O>(
    mut input: Pin<&mut I>,
    output: &mut O,
    cx: &mut Context<'_>,
    delta_limit: usize,
) -> PollResult
where
    I: futures_io::AsyncRead,
    O: bytes::BufMut,
{
    let mut rdbuf = [0u8; 8192];
    let start = output.remaining_mut();
    loop {
        let buflim = *[
            rdbuf.len(),
            output.remaining_mut(),
            delta_limit - (start - output.remaining_mut()),
        ]
        .iter()
        .min()
        .unwrap();
        match input.as_mut().poll_read(cx, &mut rdbuf[..buflim]) {
            // if we managed to read something....
            Poll::Ready(Ok(n)) if n != 0 => output.put_slice(&rdbuf[..n]),

            // assumption: if we get here, the call to poll_read failed and
            // didn't read anything
            ret => {
                return PollResult {
                    delta: start - output.remaining_mut(),
                    ret: ret_reduce(ret, buflim == 0),
                }
            }
        }
    }
}

pub fn poll_write<I, O>(input: &mut I, mut output: Pin<&mut O>, cx: &mut Context<'_>) -> PollResult
where
    I: bytes::Buf,
    O: futures_io::AsyncWrite,
{
    let start = input.remaining();
    loop {
        match output.as_mut().poll_write(cx, input.bytes()) {
            // if we managed to write something...
            Poll::Ready(Ok(n)) if n != 0 => input.advance(n),

            // assumption: if we get here, the call to poll_write failed and
            // didn't write anything
            ret => {
                return PollResult {
                    delta: start - input.remaining(),
                    ret: ret_reduce(ret, !input.has_remaining()),
                }
            }
        }
    }
}
