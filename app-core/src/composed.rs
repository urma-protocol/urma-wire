use crate::errors::ComposeError;
use crate::topics::Kind;
use urma_core::topics::Topics;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Token {
    pub kind: Kind,
    pub label: String,
    pub at: usize,
    pub len: usize,
}

impl Token {
    pub fn spelled(&self) -> String {
        format!("{}{}", self.kind.mark(), self.label)
    }

    pub fn end(&self) -> Result<usize, ComposeError> {
        advanced(self.at, self.len)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Composed {
    pub wires: Vec<Token>,
    pub hashtags: Vec<Token>,
}

pub enum Typed {
    Token(Token),
    Plain,
}

impl Composed {
    pub fn wire(&self) -> String {
        let [first, ..] = self.wires.as_slice() else {
            return String::new();
        };
        first.label.clone()
    }

    pub fn extra_wires(&self) -> Vec<String> {
        self.wires
            .iter()
            .skip(1)
            .map(|token| token.label.clone())
            .collect()
    }

    pub fn hashtag_labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = Vec::new();
        for token in &self.hashtags {
            if labels.contains(&token.label) {
                continue;
            }
            labels.push(token.label.clone());
        }
        labels.sort();
        labels.truncate(Topics::MAX_HASHTAGS);
        labels
    }

    pub fn dropped_hashtags(&self) -> Vec<String> {
        let kept = self.hashtag_labels();
        let mut dropped: Vec<String> = Vec::new();
        for token in &self.hashtags {
            if kept.contains(&token.label) || dropped.contains(&token.label) {
                continue;
            }
            dropped.push(token.label.clone());
        }
        dropped
    }
}

pub fn scan(text: &str) -> Result<Composed, ComposeError> {
    let mut composed = Composed::default();
    for (at, mark) in text.char_indices() {
        let kind = match marked(mark) {
            Marked::Label(kind) => kind,
            Marked::Text => continue,
        };
        if !opens_label(text, at) {
            continue;
        }
        let rest = &text[advanced(at, mark.len_utf8())?..];
        let raw: String = rest.chars().take_while(|c| !closes_label(*c)).collect();
        let label = match label_of(&raw) {
            Typed::Token(token) => token.label,
            Typed::Plain => continue,
        };
        let len = advanced(mark.len_utf8(), label.len())?;
        let token = Token {
            kind,
            label,
            at,
            len,
        };
        match kind {
            Kind::Wire => composed.wires.push(token),
            Kind::Hashtag => composed.hashtags.push(token),
        }
    }
    Ok(composed)
}

fn advanced(offset: usize, by: usize) -> Result<usize, ComposeError> {
    offset.checked_add(by).ok_or(ComposeError::Offset)
}

enum Marked {
    Label(Kind),
    Text,
}

fn marked(mark: char) -> Marked {
    match mark {
        '/' => Marked::Label(Kind::Wire),
        '#' => Marked::Label(Kind::Hashtag),
        _ => Marked::Text,
    }
}

fn closes_label(c: char) -> bool {
    c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';' | '!' | '?')
}

fn opens_label(text: &str, at: usize) -> bool {
    let Some(before) = text[..at].chars().next_back() else {
        return true;
    };
    before.is_whitespace() || matches!(before, '(' | '[' | '{')
}

fn label_bytes(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn label_of(raw: &str) -> Typed {
    let bare: String = raw
        .chars()
        .take_while(|c| label_bytes(*c))
        .collect::<String>()
        .to_ascii_lowercase();
    if bare.len() != raw.len() {
        return Typed::Plain;
    }
    match Topics::validate_label(&bare) {
        Ok(()) => Typed::Token(Token {
            kind: Kind::Wire,
            label: bare,
            at: 0,
            len: 0,
        }),
        Err(rejected) => {
            tracing::warn!(rejection = %rejected, "typed token is not a protocol label");
            Typed::Plain
        }
    }
}

fn word_start(head: &str) -> Result<usize, ComposeError> {
    for (offset, c) in head.char_indices().rev() {
        if closes_label(c) || matches!(c, '(' | '[' | '{') {
            return advanced(offset, c.len_utf8());
        }
    }
    Ok(0)
}

pub fn typing_at(text: &str, caret: usize) -> Result<Typed, ComposeError> {
    let caret = caret.min(text.len());
    let head = &text[..caret];
    let at = word_start(head)?;
    let word = &head[at..];
    let mut marks = word.chars();
    let kind = match marks.next() {
        Some(mark) => match marked(mark) {
            Marked::Label(kind) => kind,
            Marked::Text => return Ok(Typed::Plain),
        },
        None => return Ok(Typed::Plain),
    };
    if !opens_label(text, at) {
        return Ok(Typed::Plain);
    }
    let typed: String = marks.collect();
    if !typed.chars().all(label_bytes) {
        return Ok(Typed::Plain);
    }
    let len = caret.checked_sub(at).ok_or(ComposeError::Caret)?;
    Ok(Typed::Token(Token {
        kind,
        label: typed.to_ascii_lowercase(),
        at,
        len,
    }))
}

pub struct Accepted {
    pub text: String,
    pub caret: usize,
}

pub fn accept_at(text: &str, token: &Token, label: &str) -> Result<Accepted, ComposeError> {
    let spelled = format!("{}{label}", token.kind.mark());
    let mut replaced = String::new();
    replaced.push_str(&text[..token.at]);
    replaced.push_str(&spelled);
    let caret = advanced(token.at, spelled.len())?;
    replaced.push_str(&text[token.end()?.min(text.len())..]);
    Ok(Accepted {
        text: replaced,
        caret,
    })
}
