use crate::config::reveal_margin_when_unset;
use crate::config::REVEALED_SELECTOR;
use crate::config::REVEAL_MARGIN_PROPERTY;
use tracing::warn;

pub fn reveal(area: &web_sys::HtmlTextAreaElement) {
    let shown = match area.closest(REVEALED_SELECTOR) {
        Ok(Some(card)) => card,
        Ok(None) => area.clone().into(),
        Err(error) => {
            warn!(error = ?error, "composer card could not be located for scrolling");
            area.clone().into()
        }
    };
    let Some(window) = web_sys::window() else {
        warn!("composer cannot be revealed without a window");
        return;
    };
    let viewport = match window.inner_height() {
        Ok(viewport) => viewport,
        Err(error) => {
            warn!(error = ?error, "viewport height is unavailable");
            return;
        }
    };
    let Some(height) = viewport.as_f64() else {
        warn!("viewport height is not a number");
        return;
    };
    let box_of = shown.get_bounding_client_rect();
    let overflow = box_of.bottom() + margin(&window) - height;
    if overflow <= 0.0 {
        return;
    }
    let options = web_sys::ScrollToOptions::new();
    options.set_top(overflow);
    options.set_behavior(web_sys::ScrollBehavior::Smooth);
    window.scroll_by_with_scroll_to_options(&options);
}

fn margin(window: &web_sys::Window) -> f64 {
    let Some(document) = window.document() else {
        warn!("reveal margin cannot be measured without a document");
        return reveal_margin_when_unset();
    };
    let Some(body) = document.body() else {
        warn!("reveal margin cannot be measured without a body");
        return reveal_margin_when_unset();
    };
    let styled = match window.get_computed_style(&body) {
        Ok(styled) => styled,
        Err(error) => {
            warn!(error = ?error, "computed style is unavailable");
            return reveal_margin_when_unset();
        }
    };
    let Some(styled) = styled else {
        warn!("body carries no computed style");
        return reveal_margin_when_unset();
    };
    let stated = match styled.get_property_value(REVEAL_MARGIN_PROPERTY) {
        Ok(stated) => stated,
        Err(error) => {
            warn!(error = ?error, "reveal margin property could not be read");
            return reveal_margin_when_unset();
        }
    };
    let Some(pixels) = stated.trim().strip_suffix("px") else {
        warn!(stated = %stated, "reveal margin is not stated in pixels");
        return reveal_margin_when_unset();
    };
    match pixels.trim().parse::<f64>() {
        Ok(margin) => margin,
        Err(error) => {
            warn!(error = %error, "reveal margin is not a number");
            reveal_margin_when_unset()
        }
    }
}
