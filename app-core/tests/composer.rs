use wire_app_core::composer::{compose, Photo, PhotoError};
use wire_app_core::config::{MAX_IMAGES_PER_POST, MAX_IMAGE_URL_BYTES, MAX_POST_BYTES};
use wire_app_core::markdown::{segments, Segment};

#[test]
fn visual_photos_roundtrip_without_rewriting_manual_text() {
    let original = "Manual  ![kept](https://cdn.example/old.png)\n#Topic";
    let first = Photo::new("https://cdn.example/a(1).png", "A [photo] & <view>!_").unwrap();
    let edited = Photo::new("https://cdn.example/b.png", "Edited view").unwrap();
    let with_photo = compose(
        original,
        &[first.clone()],
        MAX_POST_BYTES,
        MAX_IMAGES_PER_POST,
    )
    .unwrap();
    assert!(with_photo.starts_with(original));
    let parsed: Vec<_> = segments(&with_photo)
        .into_iter()
        .filter_map(|part| match part {
            Segment::Image { url, alt } => Some((url, alt)),
            Segment::Text(_) => None,
        })
        .collect();
    assert_eq!(
        parsed,
        vec![
            ("https://cdn.example/old.png".into(), "kept".into()),
            (first.url, first.alt)
        ]
    );
    let changed = compose(original, &[edited], MAX_POST_BYTES, MAX_IMAGES_PER_POST).unwrap();
    assert!(changed.contains("b.png"));
    assert!(!changed.contains("a(1).png"));
    assert_eq!(
        compose(original, &[], MAX_POST_BYTES, MAX_IMAGES_PER_POST).unwrap(),
        original
    );
}

#[test]
fn url_scheme_syntax_count_and_bytes_are_bounded() {
    assert_eq!(
        Photo::new("http://cdn.example/a.png", "a"),
        Err(PhotoError::Https)
    );
    assert_eq!(
        Photo::new("https://cdn.example/a.png", "\n"),
        Err(PhotoError::Alt)
    );
    assert_eq!(
        Photo::new("https://cdn.example/a.png", "a\nb"),
        Err(PhotoError::AltControl)
    );
    assert_eq!(
        Photo::new(
            &format!("https://cdn.example/{}", "x".repeat(MAX_IMAGE_URL_BYTES)),
            "a"
        ),
        Err(PhotoError::UrlLength)
    );
    let photo = Photo::new("https://cdn.example/a.png", "a").unwrap();
    assert_eq!(
        compose(
            "text",
            &[photo.clone(), photo.clone(), photo.clone()],
            MAX_POST_BYTES,
            MAX_IMAGES_PER_POST
        ),
        Err(PhotoError::Count {
            limit: MAX_IMAGES_PER_POST
        })
    );
    assert_eq!(
        compose("x", &[photo], 8, MAX_IMAGES_PER_POST),
        Err(PhotoError::Bytes { limit: 8 })
    );
}

#[test]
fn managed_photo_must_survive_in_the_complete_markdown() {
    let photo = Photo::new("https://cdn.example/photo.png", "A photo").unwrap();
    let result = compose("```\nnotes", &[photo], MAX_POST_BYTES, MAX_IMAGES_PER_POST);
    assert!(
        matches!(result, Err(PhotoError::Context)),
        "an unclosed code fence absorbs the image: {result:?}"
    );
}
