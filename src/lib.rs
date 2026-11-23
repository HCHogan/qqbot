//! QQ's official bot API (QQ 开放平台): access tokens, the WebSocket
//! gateway, event and REST types, sending, recalling, muting and what a
//! group is called; and QQ's own markup, read and written: mentions, faces
//! and the transcripts of forwards in what comes, Markdown as QQ renders
//! it in what goes.
//!
//! It was written for Max (<https://github.com/HCHogan/max>), a group chat
//! bot, but knows nothing about it: Max maps QQ's messages to its own.

pub mod api;
pub mod connection;
pub mod content;
pub mod event;
pub mod gateway;
pub mod markdown;
pub mod message;
pub mod transcript;

pub use api::{Chat, Client, Credentials, Error};
pub use connection::run;
pub use event::{Dispatch, Event};
pub use message::{Body, Outgoing, Sent};
