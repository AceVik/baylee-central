//! The few RFC 6455 frames the push socket speaks (`docs/push-socket.md`).
//!
//! The server sends small text frames and the control frames; the client
//! sends only control frames (ping, pong, close), which RFC 6455 § 5.5 bounds
//! to 125 bytes of payload, unfragmented. So a frame from the client is at
//! most [`LONGEST`] bytes, and the reader holds no more than that: anything
//! else (a data frame, a reserved bit or opcode, an unmasked frame, a longer
//! length) ends the socket before its payload is read.
//!
//! The reader keeps what it has read in itself and awaits nothing but one
//! `read` into that buffer, so dropping its future (another branch of a
//! `select!` won) loses no byte.
use tokio::io::{AsyncRead, AsyncReadExt};

/// The longest frame a client may send: two bytes of header, the four of the
/// mask, and a control frame's 125 bytes.
pub(crate) const LONGEST: usize = 2 + 4 + 125;
/// The longest text the server sends, and the most the client reads
/// (`max_incoming_frame_size` in the Baylee client).
pub(crate) const TEXT: usize = 4096;

/// A close code (RFC 6455 § 7.4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Code(pub(crate) u16);

impl Code {
    /// Done as asked.
    pub(crate) const NORMAL: Self = Self(1000);
    /// The server stops, or the socket outlived its time.
    pub(crate) const GOING_AWAY: Self = Self(1001);
    /// A frame RFC 6455 does not allow.
    pub(crate) const PROTOCOL: Self = Self(1002);
    /// A data frame: this socket takes none.
    pub(crate) const UNSUPPORTED: Self = Self(1003);
    /// A close's reason that is not UTF-8.
    pub(crate) const INVALID_DATA: Self = Self(1007);
    /// The session ended, or the client sent faster than allowed.
    pub(crate) const POLICY: Self = Self(1008);
    /// What the socket says could not be kept true: dial again.
    pub(crate) const INTERNAL: Self = Self(1011);
}

/// What a client sent.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// A ping, with its payload, which the pong returns.
    Ping(Vec<u8>),
    Pong,
    /// The client closes, with its code if it gave one (a valid one, its
    /// reason UTF-8), which the server's close returns.
    Close(Option<Code>),
}

/// Why reading ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    /// The connection ended or failed.
    Gone,
    /// A frame not allowed here; the socket closes with this code.
    Refused(Code),
}

/// Frames from the client, read into a buffer of its own.
pub(crate) struct Reader {
    buffer: [u8; LONGEST],
    filled: usize,
}

impl Reader {
    pub(crate) const fn new() -> Self {
        Self {
            buffer: [0; LONGEST],
            filled: 0,
        }
    }

    /// The next frame. Cancel-safe: what was read stays in the reader.
    pub(crate) async fn next(
        &mut self,
        from: &mut (impl AsyncRead + Unpin),
    ) -> Result<Frame, Ended> {
        loop {
            if let Some(frame) = self.parse()? {
                return Ok(frame);
            }
            // `parse` takes every whole frame, and none is longer than the
            // buffer, so there is room whenever it asks for more.
            let read = from
                .read(&mut self.buffer[self.filled..])
                .await
                .map_err(|_| Ended::Gone)?;
            if read == 0 {
                return Err(Ended::Gone);
            }
            self.filled += read;
        }
    }

    /// The frame at the front of the buffer, taken out of it; `None` while it
    /// is not whole. Refuses as soon as the header shows a frame not allowed.
    fn parse(&mut self) -> Result<Option<Frame>, Ended> {
        if self.filled < 2 {
            return Ok(None);
        }
        let (first, second) = (self.buffer[0], self.buffer[1]);
        let fin = first & 0x80 != 0;
        let reserved = first & 0x70;
        let opcode = first & 0x0f;
        let masked = second & 0x80 != 0;
        let length = usize::from(second & 0x7f);
        // No extension was agreed, so a reserved bit is wrong on any frame;
        // and a client masks every frame (RFC 6455 § 5.1, § 5.2).
        if reserved != 0 || !masked {
            return Err(Ended::Refused(Code::PROTOCOL));
        }
        match opcode {
            // continuation, text, binary
            0x0..=0x2 => return Err(Ended::Refused(Code::UNSUPPORTED)),
            0x8..=0xa => {}
            _ => return Err(Ended::Refused(Code::PROTOCOL)),
        }
        // A control frame is whole and short (§ 5.5), which also rules out
        // 126 and 127.
        if !fin || length > 125 {
            return Err(Ended::Refused(Code::PROTOCOL));
        }
        let whole = 2 + 4 + length;
        if self.filled < whole {
            return Ok(None);
        }
        let mask = [
            self.buffer[2],
            self.buffer[3],
            self.buffer[4],
            self.buffer[5],
        ];
        let mut payload = self.buffer[6..whole].to_vec();
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
        self.buffer.copy_within(whole..self.filled, 0);
        self.filled -= whole;
        Ok(Some(match opcode {
            0x8 => Frame::Close(closing(&payload)?),
            0x9 => Frame::Ping(payload),
            _ => Frame::Pong,
        }))
    }
}

/// A close's body (RFC 6455 § 5.5.1, § 7.4): empty, or a code a peer may
/// send and a UTF-8 reason.
fn closing(payload: &[u8]) -> Result<Option<Code>, Ended> {
    let Some((code, reason)) = payload.split_first_chunk::<2>() else {
        return if payload.is_empty() {
            Ok(None)
        } else {
            Err(Ended::Refused(Code::PROTOCOL))
        };
    };
    let code = u16::from_be_bytes(*code);
    // 1004 to 1006 and 1015 are reserved or never sent; 1016 to 2999 are
    // the protocol's own, unassigned; 3000 to 4999 are the applications'.
    if !matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999) {
        return Err(Ended::Refused(Code::PROTOCOL));
    }
    if std::str::from_utf8(reason).is_err() {
        return Err(Ended::Refused(Code::INVALID_DATA));
    }
    Ok(Some(Code(code)))
}

/// A text frame of `text`, unmasked as a server's are. `None` for a text
/// longer than [`TEXT`], which no event is.
pub(crate) fn text(text: &str) -> Option<Vec<u8>> {
    let payload = text.as_bytes();
    if payload.len() > TEXT {
        return None;
    }
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.push(0x81);
    if let Ok(short) = u8::try_from(payload.len())
        && short <= 125
    {
        frame.push(short);
    } else {
        frame.push(126);
        frame.extend_from_slice(&u16::try_from(payload.len()).ok()?.to_be_bytes());
    }
    frame.extend_from_slice(payload);
    Some(frame)
}

/// A ping with no payload.
pub(crate) const PING: [u8; 2] = [0x89, 0x00];

/// The pong to a ping with `payload`: the reader takes no longer one than
/// 125 bytes, and none longer is sent.
pub(crate) fn pong(payload: &[u8]) -> Vec<u8> {
    let payload = &payload[..payload.len().min(125)];
    let mut frame = Vec::with_capacity(2 + payload.len());
    frame.push(0x8a);
    frame.push(u8::try_from(payload.len()).unwrap_or(125));
    frame.extend_from_slice(payload);
    frame
}

/// A close with `code` and no reason.
pub(crate) const fn close(code: Code) -> [u8; 4] {
    let [high, low] = code.0.to_be_bytes();
    [0x88, 0x02, high, low]
}

#[cfg(test)]
mod tests;
