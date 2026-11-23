//! A message's `content`: text, with QQ's markup for mentions and faces.
//!
//! - `<@member_openid>` mentions someone, with their name in the message's
//!   `mentions`; `<@all>` (or `<@everyone>`) everyone.
//! - `<faceType=1,faceId="306",ext="eyJ0ZXh0Ijoi54mb5rCU5Yay5aSpIn0=">` is a
//!   face, its `ext` base64 of `{"text":"牛气冲天"}`, its name.
//!
//! A `<` that starts no tag, as in `a<b`, is text.

use base64::Engine;
use base64::engine::general_purpose::STANDARD_PAD_INDIFFERENT;

/// A piece of content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// Never empty.
    Text(String),
    /// Someone, by the openid they have in `mentions`.
    At(String),
    /// Everyone.
    AtAll,
    Face(Face),
}

/// A QQ face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    /// `faceType`: 1 a face, 3 a super face, 6 a saved sticker, whose
    /// picture is in the attachments.
    pub kind: Option<u32>,
    /// `faceId`.
    pub id: String,
    /// Such as 牛气冲天; a saved sticker has none.
    pub name: Option<String>,
}

/// `content` in segments.
pub fn parse(content: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut rest = content;
    while let Some(start) = rest.find('<') {
        let Some(len) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start + 1..start + len];
        let segment = match tag.strip_prefix('@') {
            Some("all" | "everyone") => Some(Segment::AtAll),
            Some(openid) => Some(Segment::At(openid.to_owned())),
            None if tag.starts_with("faceType=") => Some(Segment::Face(face(tag))),
            None => None,
        };
        let Some(segment) = segment else {
            text.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        };
        text.push_str(&rest[..start]);
        if !text.is_empty() {
            segments.push(Segment::Text(std::mem::take(&mut text)));
        }
        segments.push(segment);
        rest = &rest[start + len + 1..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    segments
}

/// `faceType=1,faceId="306",ext="…"`.
fn face(tag: &str) -> Face {
    let field = |name: &str| {
        let rest = tag.split_once(&format!("{name}="))?.1;
        Some(match rest.strip_prefix('"') {
            Some(quoted) => quoted.split('"').next()?,
            None => rest.split(',').next()?,
        })
    };
    let name = field("ext")
        .and_then(|ext| STANDARD_PAD_INDIFFERENT.decode(ext).ok())
        .and_then(|json| serde_json::from_slice::<serde_json::Value>(&json).ok())
        .and_then(|ext| ext["text"].as_str().map(str::to_owned))
        .filter(|name| !name.is_empty());
    Face {
        kind: field("faceType").and_then(|kind| kind.parse().ok()),
        id: field("faceId").unwrap_or_default().to_owned(),
        name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup() {
        assert_eq!(
            parse(
                "<@BOT> 看<@XIAOHONG>这个<faceType=1,faceId=\"306\",ext=\"eyJ0ZXh0IjogIueJm+awlOWGsuWkqSJ9\"> a<b <@all><faceType=6,faceId=\"0\",ext=\"eyJ0ZXh0IjoiIn0=\">"
            ),
            [
                Segment::At("BOT".into()),
                Segment::Text(" 看".into()),
                Segment::At("XIAOHONG".into()),
                Segment::Text("这个".into()),
                Segment::Face(Face {
                    kind: Some(1),
                    id: "306".into(),
                    name: Some("牛气冲天".into()),
                }),
                Segment::Text(" a<b ".into()),
                Segment::AtAll,
                Segment::Face(Face {
                    kind: Some(6),
                    id: "0".into(),
                    name: None,
                }),
            ]
        );
    }
}
