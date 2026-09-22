use leptos::prelude::*;
use leptos_router::hooks::use_location;
use thaw::{Menu, MenuItem, MenuPosition, MenuTrigger};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
let timer, replay, sequence = 0;
export function stop_logo_animation() {
    clearInterval(timer);
    if (replay) window.removeEventListener("urma:logo-animate", replay);
}
export function start_logo_animation() {
    stop_logo_animation();
    replay = () => {
        if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
        const logo = document.getElementById("wire-brand-mark");
        if (logo) logo.src = "/assets/urma-wire.svg?play=" + (++sequence);
    };
    window.addEventListener("urma:logo-animate", replay);
    timer = setInterval(replay, 300000);
}
"#)]
extern "C" {
    fn start_logo_animation();
    fn stop_logo_animation();
}

#[component]
pub fn Navbar() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| start_logo_animation());
        on_cleanup(stop_logo_animation);
    }
    let location = use_location();
    view! {
        <header class="app-header">
            <a href="/" class="brand-logo" aria-label="URMA Wire home">
                <img id="wire-brand-mark" class="brand-mark" src="/assets/urma-wire.svg" alt="" width="64" height="64" />
                <span class="logo-text">"Wire"</span>
            </a>
            <nav class="header-nav" aria-label="Main navigation">
                <a href="/" class=move || if location.pathname.get() == "/" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 4h16v16H4zM8 8h8M8 12h8M8 16h4"/></svg><span>"Feed"</span>
                </a>
                <a href="/compose" class=move || if location.pathname.get() == "/compose" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 4 5 5-11 11H4v-5ZM12 7l5 5"/></svg><span>"Publish"</span>
                </a>
                <a href="/identity" class=move || if location.pathname.get() == "/identity" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="8" r="4"/><path d="M4 21v-2a8 8 0 0 1 16 0v2"/></svg><span>"Identity"</span>
                </a>
                <a href="/wallet" class=move || if location.pathname.get() == "/wallet" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 8H4V5l14-2v5M4 8v12h16V8ZM20 12h-6v4h6"/></svg><span>"Wallet"</span>
                </a>
                <a href="/connection" class=move || if location.pathname.get() == "/connection" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/></svg><span>"Connection"</span>
                </a>
                <a href="/settings" class=move || if location.pathname.get() == "/settings" { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3.5"/><path d="M12 2.5v3M12 18.5v3M21.5 12h-3M5.5 12h-3M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1M18.7 18.7l-2.1-2.1M7.4 7.4 5.3 5.3"/></svg><span>"Settings"</span>
                </a>
                <a href="/components" class=move || if location.pathname.get().starts_with("/components") { "nav-link active" } else { "nav-link" }>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/></svg><span>"Components"</span>
                </a>
            </nav>
            <div class="sidebar-note"><span class="eyebrow">"YOUR IDENTITY. YOUR WORDS."</span><p>"Signed by you."<br/>"Recorded on-chain."</p></div>
            <div class="network-switcher">
                <Menu class="network-menu" position=MenuPosition::TopStart on_select=|_| {}>
                    <MenuTrigger slot>
                        <button type="button" class="network-trigger" aria-label="Select network">
                            <img class="network-logo" src="/assets/litecoin.svg" alt="" width="16" height="16"/>
                            <span>"Litecoin"<small>"Testnet"</small></span>
                            <span class="switcher-chevron" aria-hidden="true">"⌃"</span>
                        </button>
                    </MenuTrigger>
                    <MenuItem value="ltc-testnet">
                        <img class="network-logo" src="/assets/litecoin.svg" alt="" width="16" height="16"/>"Litecoin"<small>"Testnet"</small>
                    </MenuItem>
                    <MenuItem value="ltc-mainnet" disabled=true>
                        <img class="network-logo" src="/assets/litecoin.svg" alt="" width="16" height="16"/>"Litecoin"<small>"Mainnet"</small>
                    </MenuItem>
                    <MenuItem value="btc-testnet4" disabled=true>
                        <img class="network-logo" src="/assets/bitcoin.svg" alt="" width="16" height="16"/>"Bitcoin"<small>"Testnet4"</small>
                    </MenuItem>
                    <MenuItem value="btc-mainnet" disabled=true>
                        <img class="network-logo" src="/assets/bitcoin.svg" alt="" width="16" height="16"/>"Bitcoin"<small>"Mainnet"</small>
                    </MenuItem>
                </Menu>
            </div>
        </header>
    }
}
