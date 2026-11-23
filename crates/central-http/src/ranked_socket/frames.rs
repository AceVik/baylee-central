//! RFC 6455 binary/control frame forwarding. Validate lengths and the
//! cumulative fragmented message before streaming body chunks. A native mask
//! is removed and Central chooses its own unpredictable upstream mask.
use std::{io, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    time::{Instant, timeout_at},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum From {
    Native,
    Gateway,
}

impl From {
    fn message_limit(self) -> usize {
        match self {
            Self::Native => 64 * 1024,
            Self::Gateway => 16 * 1024 * 1024,
        }
    }
    fn budget(self) -> usize {
        match self {
            Self::Native => 512 * 1024,
            Self::Gateway => 32 * 1024 * 1024,
        }
    }
}

pub(super) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid game relay frame")
}

pub(super) struct Header {
    pub(super) fin: bool,
    pub(super) opcode: u8,
    pub(super) length: usize,
    pub(super) mask: [u8; 4],
}

impl Header {
    pub(super) async fn read(r: &mut (impl AsyncRead + Unpin), from: From) -> io::Result<Self> {
        let first = r.read_u8().await?;
        tokio::time::timeout(Duration::from_secs(3), Self::remaining(r, from, first)).await?
    }

    async fn remaining(
        r: &mut (impl AsyncRead + Unpin),
        from: From,
        first: u8,
    ) -> io::Result<Self> {
        let second = r.read_u8().await?;
        let fin = first & 0x80 != 0;
        let opcode = first & 15;
        let masked = second & 0x80 != 0;
        if first & 0x70 != 0
            || masked != (from == From::Native)
            || !matches!(opcode, 0 | 2 | 8..=10)
        {
            return Err(invalid());
        }
        let marker = second & 0x7f;
        if opcode >= 8 && (!fin || marker > 125) {
            return Err(invalid());
        }
        let length = match marker {
            126 => {
                let n = u64::from(r.read_u16().await?);
                if n < 126 {
                    return Err(invalid());
                }
                n
            }
            127 => {
                let n = r.read_u64().await?;
                if u16::try_from(n).is_ok() {
                    return Err(invalid());
                }
                n
            }
            n => u64::from(n),
        };
        let length = usize::try_from(length).map_err(|_| invalid())?;
        if length > from.message_limit() {
            return Err(invalid());
        }
        let mut mask = [0; 4];
        if masked {
            r.read_exact(&mut mask).await?;
        }
        Ok(Self {
            fin,
            opcode,
            length,
            mask,
        })
    }

    fn outgoing(&self, masked: bool, key: [u8; 4]) -> Vec<u8> {
        let mut head = Vec::with_capacity(14);
        head.push((if self.fin { 0x80 } else { 0 }) | self.opcode);
        let bit = if masked { 128 } else { 0 };
        if let Ok(n @ 0..=125) = u8::try_from(self.length) {
            head.push(bit | n);
        } else if let Ok(n) = u16::try_from(self.length) {
            head.push(bit | 0x7e);
            head.extend(n.to_be_bytes());
        } else {
            head.push(bit | 0x7f);
            head.extend((self.length as u64).to_be_bytes());
        }
        if masked {
            head.extend(key);
        }
        head
    }
}

pub(super) fn mask(bytes: &mut [u8], key: [u8; 4], offset: usize) {
    for (i, b) in bytes.iter_mut().enumerate() {
        *b ^= key[(offset + i) % 4];
    }
}

struct Pieces {
    length: usize,
    frames: u16,
    deadline: Instant,
}

/// Conservatively bound both a burst and a fragmented message. A stream of
/// empty continuations or ping frames spends the same frame allowance.
pub(super) async fn forward(
    mut reader: impl AsyncRead + Unpin,
    mut writer: impl AsyncWrite + Unpin,
    from: From,
) -> io::Result<()> {
    let mut pieces: Option<Pieces> = None;
    let mut budget = (Instant::now(), 0usize, 0u16);
    let mut chunk = [0u8; 8192];
    loop {
        let now = Instant::now();
        let deadline = pieces
            .as_ref()
            .map_or(now + Duration::from_secs(600), |p| p.deadline);
        let head = timeout_at(deadline, Header::read(&mut reader, from)).await??;
        let now = Instant::now();
        if now.duration_since(budget.0) >= Duration::from_secs(1) {
            budget = (now, 0, 0);
        }
        budget.1 = budget.1.saturating_add(head.length);
        budget.2 += 1;
        if budget.1 > from.budget() || budget.2 > 512 {
            return Err(invalid());
        }
        match (head.opcode, pieces.as_mut()) {
            (0, Some(p)) => {
                p.length = p.length.saturating_add(head.length);
                p.frames += 1;
                if p.length > from.message_limit() || p.frames > 128 {
                    return Err(invalid());
                }
            }
            (2, None) if !head.fin => {
                pieces = Some(Pieces {
                    length: head.length,
                    frames: 1,
                    deadline: now + Duration::from_secs(15),
                });
            }
            (0, None) | (2, Some(_)) => return Err(invalid()),
            _ => {}
        }
        let deadline = pieces
            .as_ref()
            .map_or(now + Duration::from_secs(15), |p| p.deadline);
        let masked = from == From::Native;
        let mut key = [0; 4];
        if masked {
            getrandom::fill(&mut key).map_err(|_| io::Error::other("entropy unavailable"))?;
        }
        let remask = std::array::from_fn(|i| head.mask[i] ^ key[i]);
        timeout_at(deadline, async {
            // A remote close reason is arbitrary text. Close both transports
            // locally, without relaying an operator's message or identifiers.
            if head.opcode == 8 {
                return Ok(());
            }
            writer.write_all(&head.outgoing(masked, key)).await?;
            let mut offset = 0;
            while offset < head.length {
                let len = (head.length - offset).min(chunk.len());
                let body = &mut chunk[..len];
                reader.read_exact(body).await?;
                if masked {
                    mask(body, remask, offset);
                }
                writer.write_all(body).await?;
                offset += len;
            }
            writer.flush().await
        })
        .await??;
        if head.opcode == 8 {
            return Ok(());
        }
        if head.opcode == 0 && head.fin {
            pieces = None;
        }
    }
}
