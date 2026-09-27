pub use wire_app_core::markdown;
pub use wire_app_engine::{config, errors};

mod transport {
    include!("../src/native/media.rs");

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::new(width, height)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn response(status: u16, location: Option<&str>) -> Response {
        Response {
            status,
            location: match location {
                Some(target) => Header::Present(target.to_owned()),
                None => Header::Absent,
            },
            mime: Header::Present("image/png".into()),
            length: None,
            body: Box::new(Cursor::new(png(1, 1))),
        }
    }

    #[test]
    fn one_redirect_rechecks_and_pins_both_destinations_and_never_caches() {
        let mut resolutions = vec![];
        let mut requests = vec![];
        for _ in 0..2 {
            let result = fetch_with(
                "https://original.example/image",
                |url| {
                    resolutions.push(url.host_str().unwrap().to_owned());
                    Ok(vec![if url.host_str() == Some("original.example") {
                        "8.8.8.8:443"
                    } else {
                        "1.1.1.1:443"
                    }
                    .parse()
                    .unwrap()])
                },
                |url, addresses| {
                    requests.push((url.host_str().unwrap().to_owned(), addresses[0]));
                    Ok(if url.host_str() == Some("original.example") {
                        response(302, Some("https://redirect.example/image"))
                    } else {
                        response(200, None)
                    })
                },
            )
            .unwrap();
            assert!(result.data_url.starts_with("data:image/png;base64,"));
        }
        assert_eq!(
            resolutions,
            [
                "original.example",
                "redirect.example",
                "original.example",
                "redirect.example"
            ]
        );
        assert_eq!(requests[0].1, "8.8.8.8:443".parse().unwrap());
        assert_eq!(requests[1].1, "1.1.1.1:443".parse().unwrap());
        assert_eq!(requests.len(), 4);
    }

    #[test]
    fn private_redirect_second_redirect_and_mixed_dns_are_rejected_before_connecting() {
        let mut connections = 0;
        let error = fetch_with(
            "https://public.example/image",
            |url| {
                Ok(vec![if url.host_str() == Some("public.example") {
                    "8.8.8.8:443"
                } else {
                    "127.0.0.1:443"
                }
                .parse()
                .unwrap()])
            },
            |_, _| {
                connections += 1;
                Ok(response(302, Some("https://private.example/image")))
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("not public"));
        assert_eq!(connections, 1);
        let error = fetch_with(
            "https://public.example/image",
            |_| Ok(vec!["8.8.8.8:443".parse().unwrap()]),
            |_, _| Ok(response(302, Some("/again"))),
        )
        .unwrap_err();
        assert!(error.to_string().contains("one redirect"));
        let error = fetch_with(
            "https://public.example/image",
            |_| {
                Ok(vec![
                    "8.8.8.8:443".parse().unwrap(),
                    "10.0.0.1:443".parse().unwrap(),
                ])
            },
            |_, _| panic!("must not connect to a mixed public/private answer"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("not public"));
        assert!(fetch("http://127.0.0.1/image")
            .unwrap_err()
            .to_string()
            .contains("not public"));
    }

    #[test]
    fn special_addresses_are_not_external_image_destinations() {
        for value in [
            "0.0.0.0",
            "10.2.3.4",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:8.8.8.8",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "2002:0808:0808::1",
            "3fff::1",
        ] {
            assert!(!public_address(value.parse().unwrap()), "{value}");
        }
        for value in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public_address(value.parse().unwrap()));
        }
    }

    #[test]
    fn only_decodable_bounded_raster_images_are_rendered() {
        let bytes = png(2, 2);
        assert!(decode_image("image/png", &bytes).is_ok());
        for format in [ImageFormat::Jpeg, ImageFormat::WebP, ImageFormat::Gif] {
            let mut encoded = Cursor::new(Vec::new());
            image::RgbImage::new(2, 2)
                .write_to(&mut encoded, format)
                .unwrap();
            assert!(
                decode_image("image/png", encoded.get_ref()).is_ok(),
                "{format:?} decodes from its signature regardless of declared type"
            );
        }
        assert!(decode_image("image/jpeg", &bytes).is_ok());
        assert!(decode_image("image/svg+xml", b"<svg/>").is_err());
        assert!(decode_image("text/html", &bytes).is_err());
        assert!(decode_image("image/png", &bytes[..24]).is_err());
        assert!(decode_image("image/png", &png(4097, 1)).is_err());
        assert!(decode_image("image/png", &png(2049, 2049))
            .unwrap_err()
            .to_string()
            .contains("bounds"));
        let limit = usize::try_from(MAX_IMAGE_DOWNLOAD_BYTES).unwrap();
        assert!(decode_image("image/png", &vec![0; limit + 1]).is_err());
        let error = fetch_with(
            "https://public.example/image",
            |_| Ok(vec!["8.8.8.8:443".parse().unwrap()]),
            |_, _| {
                let mut response = response(200, None);
                response.body = Box::new(std::io::repeat(0));
                Ok(response)
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("4 MiB"));
    }

    #[test]
    fn avif_policy_matches_the_compiled_platform_codecs() {
        let avif_header = b"\0\0\0\x18ftypavif\0\0\0\0";
        assert_eq!(
            image::guess_format(avif_header).unwrap(),
            ImageFormat::Avif
        );
        #[cfg(target_os = "android")]
        {
            assert!(!SUPPORTED_IMAGE_FORMATS.contains(&ImageFormat::Avif));
            assert!(matches!(
                decode_image("image/avif", avif_header),
                Err(MediaError::Format)
            ));
        }
        #[cfg(not(target_os = "android"))]
        assert!(SUPPORTED_IMAGE_FORMATS.contains(&ImageFormat::Avif));
    }
}

#[test]
fn markdown_is_inert_and_browser_capability_fails_explicitly() {
    use markdown::{segments, Segment};
    let parts =
        segments("Hello ![drawing](https://public.example/a.png)\n<script>alert(1)</script>");
    assert!(parts.iter().any(|part| matches!(part, Segment::Image { url, alt } if url == "https://public.example/a.png" && alt == "drawing")));
    assert!(parts
        .iter()
        .any(|part| matches!(part, Segment::Text(text) if text.contains("<script>"))));
    assert!(markdown::image_url("javascript:alert(1)").is_err());
    assert!(markdown::image_url("data:image/png;base64,AAAA").is_err());
    assert!(markdown::image_url("file:///etc/passwd").is_err());
    assert!(markdown::image_url("https://user:secret@example.com/a.png").is_err());
    assert!(markdown::image_url("https://:secret@example.com/a.png").is_err());
    assert!(markdown::image_url("http://8.8.8.8/a.png").is_ok());
    assert!(
        markdown::browser_image_error("http://example.com/a.png", true).contains("mixed-content")
    );
    assert!(
        markdown::browser_image_error("https://example.com/a.png", true).contains("cannot enforce")
    );
}
