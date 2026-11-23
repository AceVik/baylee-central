//! Canonical system input shared with Baylee's protobuf game engine. Only the
//! journal service can append an entry whose source is `Authority`.
use super::CommandBytes;
use serde::{Deserialize, Serialize};

/// Visible whole seconds remaining in seat order, rounded up. Question
/// identifiers and timestamps stay inside the trusted execution boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockView {
    /// Time left for each connected player who owes an answer.
    pub decision_secs: [Option<u16>; 2],
    /// Time left for disconnected players, including a shared both-away hold.
    pub reconnect_secs: [Option<u16>; 2],
    /// The client may animate the countdown between observations.
    pub running: bool,
}

/// What elapsed while both independent game replicas were available.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Reason {
    /// The player used their decision time.
    Decision = 1,
    /// The player remained disconnected while owing an answer.
    Disconnected = 2,
}

/// A concession bound to one independently witnessed question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockEvent {
    seat: u8,
    decision: u64,
    reason: Reason,
}

impl ClockEvent {
    /// A clock can only target an assigned seat in a duel.
    #[must_use]
    pub fn new(seat: u8, decision: u64, reason: Reason) -> Option<Self> {
        (seat < 2).then_some(Self {
            seat,
            decision,
            reason,
        })
    }

    /// Seat index carried by the immutable journal input.
    #[must_use]
    pub const fn seat(self) -> u8 {
        self.seat
    }

    /// Encode Baylee v1 Envelope field 50, with the nested fields in number
    /// order. The optional question is present even when its value is zero.
    ///
    /// # Errors
    /// Returns a protocol error if the command exceeds the wire budget.
    pub fn encode(self) -> Result<CommandBytes, crate::Error> {
        if self.seat > 1 {
            return Err(crate::Error::Rejected);
        }
        let mut bytes = vec![0x92, 0x03, 0];
        if self.seat == 1 {
            bytes.extend([0x08, 1]);
        }
        bytes.push(0x10);
        let mut value = self.decision;
        while value >= 128 {
            bytes.push(value.to_le_bytes()[0] | 0x80);
            value >>= 7;
        }
        bytes.extend([value.to_le_bytes()[0], 0x18, self.reason as u8]);
        bytes[2] = u8::try_from(bytes.len() - 3).map_err(|_| crate::Error::Rejected)?;
        CommandBytes::new(bytes)
    }

    /// Read only the unique canonical encoding, including bounds on both the
    /// seat and the final varint byte. Ordinary seat actions never enter here.
    #[must_use]
    pub fn parse(body: &CommandBytes) -> Option<Self> {
        let [0x92, 0x03, length, rest @ ..] = body.as_slice() else {
            return None;
        };
        if usize::from(*length) != rest.len() {
            return None;
        }
        let (seat, rest) = rest
            .strip_prefix(&[0x08, 1])
            .map_or((0, rest), |next| (1, next));
        let [0x10, encoded @ .., 0x18, reason] = rest else {
            return None;
        };
        let reason = match reason {
            1 => Reason::Decision,
            2 => Reason::Disconnected,
            _ => return None,
        };
        let mut decision = 0;
        for (i, &byte) in encoded.iter().enumerate() {
            if i > 9 || (i == 9 && byte > 1) {
                return None;
            }
            decision |= u64::from(byte & 127) << (7 * i);
            if byte & 128 == 0 {
                return (i + 1 == encoded.len() && (i == 0 || byte != 0)).then_some(Self {
                    seat,
                    decision,
                    reason,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_engine_envelope_including_zero_and_maximal_question() {
        for seat in 0..2 {
            for decision in [0, 1, 127, 128, 16384, u64::MAX] {
                for reason in [Reason::Decision, Reason::Disconnected] {
                    let event = ClockEvent::new(seat, decision, reason).unwrap();
                    assert_eq!(ClockEvent::parse(&event.encode().unwrap()), Some(event));
                }
            }
        }
        let specimen = ClockEvent::new(1, 128, Reason::Disconnected)
            .unwrap()
            .encode()
            .unwrap();
        assert_eq!(specimen.as_slice(), &[0x92, 3, 7, 8, 1, 16, 128, 1, 24, 2]);
        for invalid in [
            &[0x92, 3, 5, 16, 128, 0, 24, 1][..],
            &[0x92, 3, 4, 16, 0, 24, 0],
        ] {
            assert!(ClockEvent::parse(&CommandBytes::new(invalid.to_vec()).unwrap()).is_none());
        }
    }
}
