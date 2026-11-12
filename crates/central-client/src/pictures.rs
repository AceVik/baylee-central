//! Profile pictures (`docs/profile-pictures.md`) for a client: the keys of
//! the pictures of public tags, a report of one, and the path a picture is
//! read at. Central serves picture requests on the website's listener; the
//! native listener refuses them until the client has a route of its own
//! (M6), so today these answer `Rejected` there. The bytes are never on this
//! wire: a client reads a picture at its path, by its random key.
use super::{Error, Session, Transport};
use central_api::picture::{PictureKey, PictureReason, TaggedPicture, key_hex};
use central_api::social::{SocialRequest, SocialResponse};

/// One of a picture's two encodings, which share its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureFile {
    /// The AVIF, 256 px.
    Avif,
    /// The PNG, 256 px: the fallback for clients without AVIF.
    Png,
}

/// The path a picture is read at, relative to Central's site: anyone may
/// read it, signed in or not, until it is replaced or removed.
#[must_use]
pub fn picture_path(key: &PictureKey, file: PictureFile) -> String {
    let extension = match file {
        PictureFile::Avif => "avif",
        PictureFile::Png => "png",
    };
    format!("/api/v1/pictures/{}.{extension}", key_hex(key))
}

impl Session {
    /// The current picture keys of `tags`, in their order; tags without a
    /// picture, unknown ones and accounts with a block between them and
    /// this one are absent. Tags outside the account's contacts and groups
    /// spend its lookup budget.
    ///
    /// # Errors
    /// `Rejected` on a listener that does not serve it, for more than
    /// [`central_api::picture::MAX_PICTURE_TAGS`] tags and for an
    /// unexpected answer; `Busy` once the lookup budget is spent.
    pub fn picture_keys(
        &self,
        transport: &impl Transport,
        tags: Vec<u32>,
    ) -> Result<Vec<TaggedPicture>, Error> {
        if tags.len() > central_api::picture::MAX_PICTURE_TAGS {
            return Err(Error::Rejected);
        }
        match self.social(transport, SocialRequest::Pictures { tags })? {
            SocialResponse::Pictures { pictures } => Ok(pictures),
            _ => Err(Error::Rejected),
        }
    }

    /// Report the picture `key` of the account `tag` to Central's
    /// administrators, with a reason and an optional note. The reported
    /// person is never told who reported it.
    ///
    /// # Errors
    /// `Rejected` for one's own picture, one that is not the account's
    /// current one or not visible, and a note out of bounds; `Conflict` for
    /// a second open report of the same picture; `Busy` past the reporting
    /// rate.
    pub fn report_picture(
        &self,
        transport: &impl Transport,
        tag: u32,
        key: PictureKey,
        reason: PictureReason,
        note: Option<String>,
    ) -> Result<(), Error> {
        if note.as_deref().is_some_and(|note| {
            !central_api::picture::valid_note(note, central_api::picture::MAX_REPORT_NOTE)
        }) {
            return Err(Error::Rejected);
        }
        let request = SocialRequest::ReportPicture {
            tag,
            key,
            reason,
            note,
        };
        match self.social(transport, request)? {
            SocialResponse::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::wire::{Identity, Request, Response};
    use central_api::{Endpoint, Token};
    use central_crypto::vault::MasterKey;
    use std::cell::RefCell;

    struct Answering {
        endpoint: Endpoint,
        answer: Vec<u8>,
        asked: RefCell<Vec<String>>,
    }

    impl Transport for Answering {
        fn endpoint(&self) -> &Endpoint {
            &self.endpoint
        }
        fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
            assert!(bearer.is_some());
            self.asked
                .borrow_mut()
                .push(String::from_utf8(central_api::encode(request)?).unwrap());
            central_api::decode(&self.answer)
        }
    }

    fn central(answer: &Response) -> Answering {
        Answering {
            endpoint: Endpoint::live(),
            answer: central_api::encode(answer).unwrap(),
            asked: RefCell::default(),
        }
    }

    fn session() -> Session {
        Session::new(
            Endpoint::live(),
            Identity {
                account: [7; 16],
                epoch: 1,
            },
            Token::generate().unwrap(),
            MasterKey::generate().unwrap(),
        )
    }

    #[test]
    fn keys_and_reports_are_asked_as_designed() {
        let found = TaggedPicture {
            tag: 0x1234,
            key: [0xab; 16],
        };
        let keys = central(&Response::Social {
            response: SocialResponse::Pictures {
                pictures: vec![found.clone()],
            },
        });
        assert_eq!(
            session().picture_keys(&keys, vec![0x1234, 0x9999]).unwrap(),
            vec![found]
        );
        assert_eq!(
            keys.asked.borrow()[0],
            r#"{"operation":"social","request":{"action":"pictures","tags":[4660,39321]}}"#
        );
        assert_eq!(
            session().picture_keys(&keys, vec![1; 301]),
            Err(Error::Rejected)
        );
        let done = central(&Response::Social {
            response: SocialResponse::Done,
        });
        session()
            .report_picture(
                &done,
                7,
                [1; 16],
                PictureReason::Private,
                Some("my address".into()),
            )
            .unwrap();
        assert!(done.asked.borrow()[0].contains(r#""action":"report_picture","tag":7"#));
        assert!(done.asked.borrow()[0].contains(r#""reason":"private","note":"my address""#));
        assert_eq!(
            session().report_picture(&done, 7, [1; 16], PictureReason::Other, Some(" ".into())),
            Err(Error::Rejected)
        );
        // a picture's answer where a report's is due is refused
        assert_eq!(
            session().report_picture(&keys, 7, [1; 16], PictureReason::Other, None),
            Err(Error::Rejected)
        );
    }

    #[test]
    fn a_picture_s_path_names_its_key_and_encoding_only() {
        let key = [0x0f; 16];
        assert_eq!(
            picture_path(&key, PictureFile::Avif),
            format!("/api/v1/pictures/{}.avif", "0f".repeat(16))
        );
        assert_eq!(
            picture_path(&key, PictureFile::Png),
            format!("/api/v1/pictures/{}.png", "0f".repeat(16))
        );
    }
}
