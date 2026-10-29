//! Talking to a Philips Hue Bridge over its local API (CLIP v2): finding and
//! pairing a bridge, following its event stream, and turning user actions
//! into requests. Independent of any UI toolkit.

pub mod bridge;
pub mod color;
pub mod config;
pub mod demo;
pub mod discovery;
pub mod home;
pub mod pinning;
pub mod service;

pub use home::{Command, Snapshot};
pub use service::{Request, Service, Update};
