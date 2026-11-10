# qqbot

A Rust client for QQ's official bot API (QQ 开放平台): access tokens, the
WebSocket gateway with heartbeats, Resume and reconnection, the event and
REST types, and sending, recalling and muting in groups and private chats,
with pictures and files, and a group's name.

It covers what [Max](https://github.com/HCHogan/max) uses, and was split
out of that repository with its history. Only the events Max handles have
types; the rest arrive as raw JSON.

```sh
export QQBOT_APP_ID=… QQBOT_SECRET=…
cargo run --example listen
echo 早 | cargo run --example send -- group <group_openid>
```

Use it from Cargo by its Git commit:

```toml
qqbot = { git = "https://github.com/HCHogan/qqbot", rev = "…" }
```

MIT licensed.
