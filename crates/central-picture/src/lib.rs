//! Profile pictures (`docs/profile-pictures.md`): the formats Central takes,
//! the protocol between the service and its picture worker, the worker's
//! client, and the checks the service makes of every answer before it
//! stores anything.
//!
//! No decoder or encoder is linked here. They live in
//! `central-picture-codec`, which only the worker binary links, so the
//! service that holds the database password never parses a stranger's
//! picture (`docs/dependency-proposal/profile-picture.md` §6.2).
#![forbid(unsafe_code)]

mod answer;
mod format;
mod protocol;
mod worker;

pub use answer::{avif_framing, digest, png_master};
pub use format::{Format, MAX_UPLOAD};
pub use protocol::{
    MAX_ANSWER, MAX_REQUEST, Output, Refusal, answer, read_answer, read_request, request,
};
pub use worker::{Converted, TIME, Worker};

/// The side of every stored picture, in pixels: AVIF and PNG alike.
pub const SIDE: u32 = 256;
/// The largest AVIF Central stores (the schema's bound).
pub const MAX_AVIF: usize = 163_840;
/// The largest PNG master Central stores (the schema's bound). A 256²
/// RGBA picture of noise needs about 263 KB.
pub const MAX_PNG: usize = 327_680;
