//! QQ's official bot API (QQ 开放平台): access tokens, the WebSocket
//! gateway, event and REST types, sending, recalling and muting.
//!
//! It was written for Max (<https://github.com/HCHogan/max>), a group chat
//! bot, but knows nothing about it: Max maps QQ's messages to its own.

pub mod api;
pub mod connection;
pub mod event;
pub mod gateway;
pub mod message;

pub use api::{Chat, Client, Credentials, Error};
pub use connection::run;
pub use event::{Dispatch, Event};
pub use message::{Body, Outgoing, Sent};
