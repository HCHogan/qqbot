//! The transcripts QQ writes in place of messages it doesn't send: a
//! merged forward's body ([`Message::forward`]), and a quoted message that
//! quotes another or holds a forward ([`Element::dump`]).
//!
//! ```text
//! [群聊的聊天记录]                      (a forward's title; a dump has none)
//! === 消息 1 ===
//! [消息内容] its text, over as many lines as it has
//! [发送者] a nickname                 (in a forward)
//! [消息类型] 合并转发消息              (or 引用消息, 卡片消息; none for text)
//! [卡片消息] 小程序 摘要:…             (a card's summary, on one line)
//! [附件1] 类型:图片 文件名:….jpg 尺寸:1671x939 大小:166.4KB URL:https://…
//! [关联消息]
//! --- 第1条 ---
//!     [消息内容] what it quotes, or a message of the forward it is
//!     [消息类型] 引用消息
//! ```
//!
//! A related message's fields are indented four spaces deeper, and so are
//! its text's later lines. Nothing in a message's text is escaped, so a
//! line in it that looks like a field is read as one.
//!
//! [`Message::forward`]: crate::event::Message::forward
//! [`Element::dump`]: crate::event::Element::dump

/// A transcript's messages, and its title if it has one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    /// `群聊的聊天记录`, or whose chat it was, such as `小明和小红的聊天记录`.
    pub title: Option<String>,
    pub messages: Vec<Entry>,
}

/// One message of a transcript.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The sender's nickname; a dump doesn't say.
    pub sender: Option<String>,
    /// Its text as QQ writes it, faces as `<faceType=…>` tags, without the
    /// transcript's indentation. None for a forward, whose title is in its
    /// kind.
    pub text: Option<String>,
    pub kind: Kind,
    /// Its pictures, videos and files.
    pub files: Vec<File>,
    /// What it quotes, as much as QQ shows of it.
    pub quoted: Option<Box<Entry>>,
}

/// What a message is, by `[消息类型]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// No `[消息类型]`: text, pictures or files.
    #[default]
    Plain,
    /// 引用消息: a message that quotes, or, among a forward's related
    /// messages, the one quoted.
    Quote,
    /// 卡片消息: its text is the card's fields, one a line.
    Card,
    /// 合并转发消息: a forward inside this one. QQ leaves its messages out
    /// at times, and then only its title is known.
    Forward(Transcript),
    Other(String),
}

/// A picture, video or file, by its `[附件N]` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub kind: FileKind,
    /// QQ's name for it: a hash for a picture, the file's own for a file.
    pub name: Option<String>,
    /// Where to download it, briefly: QQ's links expire. Some files come
    /// without one.
    pub url: Option<String>,
}

/// `类型:` of an `[附件N]` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// 图片.
    Picture,
    /// 动图: an animated picture, or a saved sticker.
    Animated,
    /// 视频.
    Video,
    /// 语音.
    Voice,
    /// 文件.
    File,
    Other(String),
}

impl Transcript {
    /// Reads a transcript; None if `text` isn't one.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text
            .split('\n')
            .map(|line| line.trim_end_matches('\r'))
            .peekable();
        let first = lines.peek()?.trim();
        let title = if is_header(first, "=== 消息 ", " ===") {
            None
        } else {
            let title = first.strip_prefix('[')?.strip_suffix(']')?.to_owned();
            lines.next();
            Some(title)
        };
        let messages: Vec<Entry> = read(lines).into_iter().map(Raw::entry).collect();
        (!messages.is_empty()).then_some(Self { title, messages })
    }
}

/// A message as read, before its related messages are told apart.
#[derive(Default)]
struct Raw {
    sender: Option<String>,
    text: Option<String>,
    kind: Option<String>,
    files: Vec<File>,
    related: Vec<Raw>,
}

impl Raw {
    /// A forward's related messages are its own; anything else's is what
    /// it quotes.
    fn entry(self) -> Entry {
        let mut related = self.related.into_iter().map(Self::entry);
        match self.kind.as_deref() {
            Some("合并转发消息") => Entry {
                sender: self.sender,
                text: None,
                kind: Kind::Forward(Transcript {
                    title: self
                        .text
                        .as_deref()
                        .and_then(|text| text.trim().strip_prefix('[')?.strip_suffix(']'))
                        .map(str::to_owned),
                    messages: related.collect(),
                }),
                files: self.files,
                quoted: None,
            },
            kind => Entry {
                sender: self.sender,
                kind: match kind {
                    None => Kind::Plain,
                    Some("引用消息") => Kind::Quote,
                    Some("卡片消息") => Kind::Card,
                    Some(other) => Kind::Other(other.to_owned()),
                },
                text: self.text,
                files: self.files,
                quoted: related.next().map(Box::new),
            },
        }
    }
}

/// The messages, line by line: one `d` deep has its fields indented `d`
/// times four spaces, and `[关联消息]` holds messages one deeper.
fn read<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Raw> {
    let mut top: Vec<Raw> = Vec::new();
    // The message being read: its index at the top, then among the related
    // messages of each one on the way.
    let mut path: Vec<usize> = Vec::new();
    let mut in_text = false;
    for line in lines {
        let trimmed = line.trim();
        if is_header(trimmed, "=== 消息 ", " ===") {
            top.push(Raw::default());
            path = vec![top.len() - 1];
            in_text = false;
            continue;
        }
        if is_header(trimmed, "--- 第", "条 ---") {
            let depth = indent(line) / 4 + 1;
            if depth > path.len() {
                continue;
            }
            path.truncate(depth);
            let Some(parent) = at(&mut top, &path) else {
                continue;
            };
            parent.related.push(Raw::default());
            path.push(parent.related.len() - 1);
            in_text = false;
            continue;
        }
        let Some(raw) = at(&mut top, &path) else {
            continue;
        };
        let prefix = "    ".repeat(path.len() - 1);
        let own = line
            .strip_prefix(prefix.as_str())
            .unwrap_or(line.trim_start());
        if let Some(text) = own.strip_prefix("[消息内容]") {
            raw.text = Some(text.strip_prefix(' ').unwrap_or(text).to_owned());
            in_text = true;
        } else if let Some(name) = own.strip_prefix("[发送者] ") {
            raw.sender = Some(name.trim().to_owned()).filter(|name| !name.is_empty());
            in_text = false;
        } else if let Some(kind) = own.strip_prefix("[消息类型] ") {
            raw.kind = Some(kind.trim().to_owned());
            in_text = false;
        } else if own.starts_with("[关联消息]") || own.starts_with("[卡片消息] ") {
            in_text = false;
        } else if let Some(file) = file(own) {
            raw.files.push(file);
            in_text = false;
        } else if in_text && let Some(text) = &mut raw.text {
            text.push('\n');
            text.push_str(own);
        }
    }
    top.iter_mut().for_each(trim);
    top
}

/// The message `path` leads to; None before the first.
fn at<'r>(top: &'r mut [Raw], path: &[usize]) -> Option<&'r mut Raw> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(top.get_mut(*first)?, |raw, i| raw.related.get_mut(*i))
}

/// A text ends where the next field starts, so blank lines before that
/// are the transcript's.
fn trim(raw: &mut Raw) {
    if let Some(text) = &mut raw.text {
        text.truncate(text.trim_end().len());
    }
    raw.related.iter_mut().for_each(trim);
}

fn is_header(line: &str, start: &str, end: &str) -> bool {
    line.strip_prefix(start)
        .and_then(|rest| rest.strip_suffix(end))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// `[附件1] 类型:图片 文件名:….jpg 尺寸:1671x939 大小:166.4KB URL:https://…`:
/// a file's name may hold spaces, the URL none.
fn file(line: &str) -> Option<File> {
    let (n, rest) = line.strip_prefix("[附件")?.split_once("] ")?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (rest, url) = match rest.rsplit_once(" URL:") {
        Some((rest, url)) => (rest, Some(url.trim()).filter(|url| !url.is_empty())),
        None => (rest, None),
    };
    let kind = rest.strip_prefix("类型:")?.split(' ').next()?;
    let name = rest.split_once(" 文件名:").map(|(_, name)| {
        [" 尺寸:", " 大小:"]
            .iter()
            .filter_map(|field| name.find(field))
            .min()
            .map_or(name, |end| &name[..end])
            .to_owned()
    });
    Some(File {
        kind: match kind {
            "图片" => FileKind::Picture,
            "动图" => FileKind::Animated,
            "视频" => FileKind::Video,
            "语音" => FileKind::Voice,
            "文件" => FileKind::File,
            other => FileKind::Other(other.to_owned()),
        },
        name,
        url: url.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(name: &str, url: &str) -> File {
        File {
            kind: FileKind::Picture,
            name: Some(name.into()),
            url: Some(url.into()),
        }
    }

    /// Shaped like the forwards QQ sent in November 2026, with made-up
    /// names and links.
    #[test]
    fn a_forward() {
        let transcript = Transcript::parse(
            "[群聊的聊天记录]
=== 消息 1 ===
[消息内容] 看这个
第二行
[发送者] 小明
[附件1] 类型:图片 文件名:A1.jpg 尺寸:10x10 大小:1.0KB URL:https://example.com/a1?rkey=x
[附件2] 类型:文件 文件名:报告 终版.pdf 大小:5.8MB

=== 消息 2 ===
[消息内容] 好家伙
[发送者] 小红
[关联消息]
--- 第1条 ---
    [消息内容] 看这个
    [消息类型] 引用消息

=== 消息 3 ===
[消息内容] [群聊的聊天记录]
[发送者] 小明
[消息类型] 合并转发消息
[关联消息]
--- 第1条 ---
    [消息内容] 早
    def f():
        pass
    [发送者] 小刚

=== 消息 4 ===
[消息内容] [卡片消息] 小程序
title: 一个视频
[发送者] 小红
[消息类型] 卡片消息
[卡片消息] 小程序 title:一个视频

",
        )
        .expect("a transcript");
        assert_eq!(transcript.title.as_deref(), Some("群聊的聊天记录"));
        assert_eq!(
            transcript.messages,
            [
                Entry {
                    sender: Some("小明".into()),
                    text: Some("看这个\n第二行".into()),
                    files: vec![
                        picture("A1.jpg", "https://example.com/a1?rkey=x"),
                        File {
                            kind: FileKind::File,
                            name: Some("报告 终版.pdf".into()),
                            url: None,
                        },
                    ],
                    ..Entry::default()
                },
                Entry {
                    sender: Some("小红".into()),
                    text: Some("好家伙".into()),
                    quoted: Some(Box::new(Entry {
                        text: Some("看这个".into()),
                        kind: Kind::Quote,
                        ..Entry::default()
                    })),
                    ..Entry::default()
                },
                Entry {
                    sender: Some("小明".into()),
                    kind: Kind::Forward(Transcript {
                        title: Some("群聊的聊天记录".into()),
                        messages: vec![Entry {
                            sender: Some("小刚".into()),
                            text: Some("早\ndef f():\n    pass".into()),
                            ..Entry::default()
                        }],
                    }),
                    ..Entry::default()
                },
                Entry {
                    sender: Some("小红".into()),
                    text: Some("[卡片消息] 小程序\ntitle: 一个视频".into()),
                    kind: Kind::Card,
                    ..Entry::default()
                },
            ]
        );
    }

    /// A quoted message that quotes another: no title, no sender.
    #[test]
    fn a_dump() {
        let transcript = Transcript::parse(
            "=== 消息 1 ===
[消息内容] 你看
[消息类型] 引用消息
[附件1] 类型:图片 文件名:0FD9.jpg 尺寸:3072x4096 大小:8.2MB URL:https://example.com/0fd9
[关联消息]
--- 第1条 ---
    [消息内容] 早",
        )
        .expect("a transcript");
        assert_eq!(
            transcript,
            Transcript {
                title: None,
                messages: vec![Entry {
                    text: Some("你看".into()),
                    kind: Kind::Quote,
                    files: vec![picture("0FD9.jpg", "https://example.com/0fd9")],
                    quoted: Some(Box::new(Entry {
                        text: Some("早".into()),
                        ..Entry::default()
                    })),
                    ..Entry::default()
                }],
            }
        );
    }

    #[test]
    fn not_a_transcript() {
        assert_eq!(Transcript::parse("[图片]"), None);
        assert_eq!(Transcript::parse("早"), None);
    }
}
