use crate::composed::{scan, Token};
use crate::markdown::{segments, Segment};
use crate::topics::Kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Label { written: String, spelled: String },
    Image { url: String, alt: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub pieces: Vec<Piece>,
    pub unwritten: Vec<String>,
}

pub fn body(text: &str, wire: &str, hashtags: &[String]) -> Body {
    let mut pieces = Vec::new();
    for segment in segments(text) {
        match segment {
            Segment::Text(plain) => pieces.extend(woven(&plain, wire, hashtags)),
            Segment::Image { url, alt } => pieces.push(Piece::Image { url, alt }),
        }
    }
    let unwritten = carried(wire, hashtags)
        .into_iter()
        .filter(|spelled| !written(&pieces, spelled))
        .collect();
    Body { pieces, unwritten }
}

fn carried(wire: &str, hashtags: &[String]) -> Vec<String> {
    let mut spelled = Vec::new();
    if !wire.is_empty() {
        spelled.push(format!("{}{wire}", Kind::Wire.mark()));
    }
    for tag in hashtags {
        spelled.push(format!("{}{tag}", Kind::Hashtag.mark()));
    }
    spelled
}

fn written(pieces: &[Piece], spelled: &str) -> bool {
    pieces.iter().any(|piece| match piece {
        Piece::Label { spelled: held, .. } => held == spelled,
        Piece::Text(_plain) => false,
        Piece::Image { .. } => false,
    })
}

fn woven(plain: &str, wire: &str, hashtags: &[String]) -> Vec<Piece> {
    let composed = match scan(plain) {
        Ok(composed) => composed,
        Err(error) => {
            tracing::warn!(error = %error, "record text could not be read for labels");
            return vec![Piece::Text(plain.to_owned())];
        }
    };
    let mut tokens: Vec<Token> = composed
        .wires
        .into_iter()
        .chain(composed.hashtags)
        .filter(|token| carries(token, wire, hashtags))
        .collect();
    tokens.sort_by_key(|token| token.at);
    let mut pieces = Vec::new();
    let mut from = 0;
    for token in tokens {
        let end = token.at + token.len;
        let (Some(before), Some(written)) = (plain.get(from..token.at), plain.get(token.at..end))
        else {
            continue;
        };
        if !before.is_empty() {
            pieces.push(Piece::Text(before.to_owned()));
        }
        pieces.push(Piece::Label {
            written: written.to_owned(),
            spelled: token.spelled(),
        });
        from = end;
    }
    let Some(rest) = plain.get(from..) else {
        return pieces;
    };
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest.to_owned()));
    }
    pieces
}

fn carries(token: &Token, wire: &str, hashtags: &[String]) -> bool {
    match token.kind {
        Kind::Wire => !wire.is_empty() && token.label == wire,
        Kind::Hashtag => hashtags.iter().any(|tag| *tag == token.label),
    }
}
