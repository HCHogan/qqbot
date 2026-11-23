//! Markdown as QQ renders it, for [`Body::Markdown`].
//!
//! - A mention is `<qqbot-at-user id="member_openid" />`, shown as @ and
//!   the member's group card. There are none in private chats.
//! - QQ renders LaTeX `$…$`, `\(…\)` and `$$…$$` but not `\[…\]`, which
//!   is written as `$$…$$`. In code it is as written: a code span, between
//!   runs of as many backticks, or a block between fences of three or more.
//!
//! [`Body::Markdown`]: crate::message::Body::Markdown

use std::ops::Range;

/// A Markdown message being written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Markdown {
    text: String,
}

impl Markdown {
    pub fn new() -> Self {
        Self::default()
    }

    /// Markdown as written.
    pub fn text(&mut self, text: &str) -> &mut Self {
        self.text.push_str(text);
        self
    }

    /// Mentions a member of the group, by their `member_openid`.
    pub fn at(&mut self, member_openid: &str) -> &mut Self {
        self.text
            .push_str(&format!("<qqbot-at-user id=\"{member_openid}\" />"));
        self
    }

    /// The message, as QQ renders it.
    pub fn finish(self) -> String {
        let code = code(&self.text);
        let mut out = String::with_capacity(self.text.len());
        let mut from = 0;
        for (at, _) in self.text.match_indices('\\') {
            if matches!(self.text.as_bytes().get(at + 1), Some(b'[' | b']'))
                && !code.iter().any(|span| span.contains(&at))
            {
                out.push_str(&self.text[from..at]);
                out.push_str("$$");
                from = at + 2;
            }
        }
        out.push_str(&self.text[from..]);
        out
    }
}

/// Where `text` is code: fenced blocks, fences included, and code spans,
/// backticks included.
fn code(text: &str) -> Vec<Range<usize>> {
    let mut code = Vec::new();
    let mut fence: Option<usize> = None;
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        let ticks = ticks(line.trim_start_matches(' '));
        match fence {
            Some(open) => {
                code.push(start..end);
                if ticks >= open && line.trim().len() == ticks {
                    fence = None;
                }
            }
            None if ticks >= 3 => {
                fence = Some(ticks);
                code.push(start..end);
            }
            None => code.extend(
                spans(line)
                    .into_iter()
                    .map(|span| start + span.start..start + span.end),
            ),
        }
        start = end;
    }
    code
}

/// The length of the run of backticks `s` starts with.
fn ticks(s: &str) -> usize {
    s.len() - s.trim_start_matches('`').len()
}

/// The code spans in a line: a run of backticks up to the next run of as
/// many.
fn spans(line: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut i = 0;
    while let Some(at) = line[i..].find('`').map(|at| i + at) {
        let open = ticks(&line[at..]);
        let mut j = at + open;
        let mut close = None;
        while let Some(next) = line[j..].find('`').map(|next| j + next) {
            let run = ticks(&line[next..]);
            if run == open {
                close = Some(next + run);
                break;
            }
            j = next + run;
        }
        match close {
            Some(end) => {
                spans.push(at..end);
                i = end;
            }
            None => i = at + open,
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(text: &str) -> String {
        let mut markdown = Markdown::new();
        markdown.text(text);
        markdown.finish()
    }

    #[test]
    fn display_math_outside_code() {
        let mut markdown = Markdown::new();
        markdown.at("XIAOMING").text(" 在，\\[x^2\\]");
        assert_eq!(
            markdown.finish(),
            "<qqbot-at-user id=\"XIAOMING\" /> 在，$$x^2$$"
        );
        assert_eq!(
            written("`grep '\\['` 和 ``a`\\[`` \\[y\\]"),
            "`grep '\\['` 和 ``a`\\[`` $$y$$"
        );
        // A fence closes at a run as long or longer, alone on its line.
        assert_eq!(
            written("```\n\\[\n```` \n\\[z\\]"),
            "```\n\\[\n```` \n$$z$$"
        );
        assert_eq!(written("````\n```\n\\[\n````"), "````\n```\n\\[\n````");
        assert_eq!(written("`没闭合 \\[x\\]"), "`没闭合 $$x$$");
    }
}
