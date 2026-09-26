use wire_app_core::body::{body, Piece};

fn tags(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|label| (*label).to_owned()).collect()
}

fn text(written: &str) -> Piece {
    Piece::Text(written.to_owned())
}

fn label(written: &str, spelled: &str) -> Piece {
    Piece::Label {
        written: written.to_owned(),
        spelled: spelled.to_owned(),
    }
}

#[test]
fn a_hashtag_written_in_the_text_is_not_repeated_as_a_chip() {
    let read = body("ce prost esti\n#prostule", "", &tags(&["prostule"]));
    assert_eq!(
        read.pieces,
        vec![text("ce prost esti\n"), label("#prostule", "#prostule"), text("\n")]
    );
    assert!(read.unwritten.is_empty());
}

#[test]
fn labels_added_only_as_fields_stay_chips() {
    let read = body("Leave Shordan ALONE !!!", "romania", &tags(&["presedintelemeu"]));
    assert_eq!(read.pieces, vec![text("Leave Shordan ALONE !!!\n")]);
    assert_eq!(read.unwritten, tags(&["/romania", "#presedintelemeu"]));
}

#[test]
fn the_written_spelling_is_kept_and_follows_the_carried_label() {
    let read = body("found #Evidence here", "", &tags(&["evidence"]));
    assert_eq!(
        read.pieces,
        vec![text("found "), label("#Evidence", "#evidence"), text(" here\n")]
    );
    assert!(read.unwritten.is_empty());
}

#[test]
fn only_labels_the_record_carries_become_labels() {
    let read = body("/rosint then /osint #leak #noise", "rosint", &tags(&["leak"]));
    assert_eq!(
        read.pieces,
        vec![
            label("/rosint", "/rosint"),
            text(" then /osint "),
            label("#leak", "#leak"),
            text(" #noise\n"),
        ]
    );
    assert!(read.unwritten.is_empty());
}

#[test]
fn a_mix_keeps_chips_only_for_what_the_text_does_not_say() {
    let read = body("tagged #leak inline", "rosint", &tags(&["archive", "leak"]));
    assert_eq!(read.unwritten, tags(&["/rosint", "#archive"]));
}

#[test]
fn text_that_does_not_read_as_a_label_keeps_its_chip() {
    let read = body("see https://host/page#section and #leak.", "", &tags(&["leak", "section"]));
    assert_eq!(
        read.pieces,
        vec![text("see https://host/page#section and #leak.\n")]
    );
    assert_eq!(read.unwritten, tags(&["#leak", "#section"]));
}

#[test]
fn formatting_around_a_label_does_not_hide_it() {
    let read = body("this is **#urgent** now", "", &tags(&["urgent"]));
    assert_eq!(
        read.pieces,
        vec![text("this is "), label("#urgent", "#urgent"), text(" now\n")]
    );
}

#[test]
fn images_keep_their_place_between_pieces() {
    let read = body("#map ![port](https://host/port.png) after", "", &tags(&["map"]));
    assert_eq!(
        read.pieces,
        vec![
            label("#map", "#map"),
            text(" "),
            Piece::Image {
                url: "https://host/port.png".to_owned(),
                alt: "port".to_owned(),
            },
            text(" after\n"),
        ]
    );
}
