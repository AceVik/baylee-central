//! The picture worker's codecs against valid and hostile files on every
//! input path (`docs/profile-pictures.md` §"Tests"), and the worker binary
//! behind the service's own client. Every file is synthetic.
#![forbid(unsafe_code)]

mod avif;
mod formats;
mod mutations;
mod pngs;
mod rasters;
mod support;
mod svg;
mod worker;
