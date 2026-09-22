use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use thaw::{ConfigProvider, Theme};

use crate::{
    components::Navbar,
    pages::{
        ComponentsPage, ComposePage, ConnectionPage, FeedPage, IdentityPage, PostDetailPage,
        ProfilePage, SettingsPage,
    },
    platform::default_bridge,
    state::AppState,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(AppState::new());
    provide_context(default_bridge());

    view! {
        <Title text="URMA Wire — Social journalism" />
        <ConfigProvider theme=RwSignal::new(Theme::dark())>
        <Router>
            <div class="app-layout">
                <Navbar />
                <main class="main-content">
                    <div id="network-content">
                    <Routes fallback=|| view! {
                        <div class="empty-state not-found">
                            <span class="empty-icon">"🔍"</span>
                            <h3>"404 — Page Not Found"</h3>
                            <p>"The requested route does not exist in URMA Wire."</p>
                        </div>
                    }>
                        <Route path=path!("/") view=FeedPage />
                        <Route path=path!("/post/:txid") view=PostDetailPage />
                        <Route path=path!("/profile/:author") view=ProfilePage />
                        <Route path=path!("/compose") view=ComposePage />
                        <Route path=path!("/components") view=ComponentsPage />
                        <Route path=path!("/settings") view=SettingsPage />
                        <Route path=path!("/wallet") view=SettingsPage />
                        <Route path=path!("/identity") view=IdentityPage />
                        <Route path=path!("/connection") view=ConnectionPage />
                    </Routes>
                    </div>
                </main>
                <footer class="app-footer">
                    <div class="footer-content">
                        <span>"URMA Wire" <span class="footer-separator">" / "</span> "Public words. Verified origin."</span>
                        <span class="footer-authority">"V0 · Litecoin Testnet"</span>
                    </div>
                </footer>
            </div>
        </Router>
        </ConfigProvider>
    }
}
