//! The gungraun-runner library

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]

#[cfg(feature = "api")]
pub mod api;
#[cfg(feature = "runner")]
pub mod error;
#[cfg(feature = "runner")]
pub mod runner;
#[cfg(feature = "runner")]
pub mod serde;
#[cfg(feature = "runner")]
pub mod util;
