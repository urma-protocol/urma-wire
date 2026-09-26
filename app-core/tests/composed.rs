use wire_app_core::composed::{accept_at, scan, typing_at, Composed, Token, Typed};
use wire_app_core::topics::Kind;

fn read(text: &str) -> Composed {
    match scan(text) {
        Ok(composed) => composed,
        Err(error) => panic!("scanning {text:?} failed: {error}"),
    }
}

fn typed(text: &str, caret: usize) -> Token {
    match typing_at(text, caret) {
        Ok(Typed::Token(token)) => token,
        Ok(Typed::Plain) => panic!("expected a token in {text:?} at {caret}"),
        Err(error) => panic!("reading the caret in {text:?} failed: {error}"),
    }
}

fn plain(text: &str, caret: usize) -> bool {
    match typing_at(text, caret) {
        Ok(Typed::Plain) => true,
        Ok(Typed::Token(token)) => panic!("expected plain text, read {}", token.spelled()),
        Err(error) => panic!("reading the caret in {text:?} failed: {error}"),
    }
}

#[test]
fn reads_wire_and_hashtags_from_body_text() {
    let composed = read("looking at /rosint with #investigation and #documents");
    assert_eq!(composed.wire(), "rosint");
    assert_eq!(
        composed.hashtag_labels(),
        vec!["documents".to_owned(), "investigation".to_owned()]
    );
    assert!(composed.extra_wires().is_empty());
}

#[test]
fn urls_and_paths_never_become_wires() {
    for text in [
        "see https://host/testnet/api for the source",
        "the file at src/main.rs compiles",
        "read http://example.org/rosint now",
        "a link (https://host/path) in brackets",
        "fragment https://host/page#section here",
    ] {
        let composed = read(text);
        assert_eq!(composed.wire(), "", "text read a wire: {text}");
        assert!(
            composed.hashtag_labels().is_empty(),
            "text read a hashtag: {text}"
        );
    }
}

#[test]
fn a_bracketed_label_still_reads() {
    let composed = read("channel (/rosint) and [#leak]");
    assert_eq!(composed.wire(), "rosint");
    assert_eq!(composed.hashtag_labels(), vec!["leak".to_owned()]);
}

#[test]
fn sentence_punctuation_ends_a_label_but_paths_still_do_not_read() {
    assert_eq!(read("posted on /rosint, then left").wire(), "rosint");
    assert_eq!(read("is it /rosint?").wire(), "rosint");
    assert_eq!(
        read("tagged #leak; done").hashtag_labels(),
        vec!["leak".to_owned()]
    );
    assert_eq!(read("/rosint/sub is a path").wire(), "");
    assert_eq!(read("/rosint.txt is a file").wire(), "");
}

#[test]
fn only_the_first_wire_applies_and_the_rest_are_reported() {
    let composed = read("/rosint then /osint and /leaks");
    assert_eq!(composed.wire(), "rosint");
    assert_eq!(
        composed.extra_wires(),
        vec!["osint".to_owned(), "leaks".to_owned()]
    );
}

#[test]
fn hashtags_beyond_the_protocol_cap_are_reported_not_dropped_silently() {
    let composed = read("#a1 #b2 #c3 #d4 #e5 #f6 #g7 #h8 #i9 #j0");
    assert_eq!(composed.hashtag_labels().len(), 8);
    assert_eq!(
        composed.dropped_hashtags(),
        vec!["i9".to_owned(), "j0".to_owned()]
    );
}

#[test]
fn repeated_hashtags_collapse_once() {
    let composed = read("#leak #leak #leak");
    assert_eq!(composed.hashtag_labels(), vec!["leak".to_owned()]);
    assert!(composed.dropped_hashtags().is_empty());
}

#[test]
fn labels_are_lowercased_and_invalid_ones_stay_text() {
    assert_eq!(read("/ROSINT").wire(), "rosint");
    assert_eq!(read("/-nope").wire(), "");
    assert_eq!(read("a / b # c").wire(), "");
    let long = format!("/{}", "a".repeat(33));
    assert_eq!(read(&long).wire(), "");
}

#[test]
fn typing_offers_the_token_under_the_caret() {
    let text = "talking about /ros";
    let token = typed(text, text.len());
    assert_eq!(token.kind, Kind::Wire);
    assert_eq!(token.label, "ros");

    let text = "tagging #inv";
    let token = typed(text, text.len());
    assert_eq!(token.kind, Kind::Hashtag);
    assert_eq!(token.label, "inv");

    assert_eq!(typed("go /", 4).label, "");
    assert!(plain("plain words", 11));
    assert!(plain("https://host/pa", 15));
}

#[test]
fn accepting_a_suggestion_replaces_only_that_token() {
    let text = "look at /ros for more";
    let token = typed(text, 12);
    let accepted = match accept_at(text, &token, "rosint") {
        Ok(accepted) => accepted,
        Err(error) => panic!("accepting the suggestion failed: {error}"),
    };
    assert_eq!(accepted.text, "look at /rosint for more");
    assert_eq!(accepted.caret, 15);
    assert_eq!(&accepted.text[..accepted.caret], "look at /rosint");
}
