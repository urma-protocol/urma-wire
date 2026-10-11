use crate::components::Missing;
use crate::components::Navbar;
use crate::pages::ComposePage;
use crate::pages::ConnectionPage;
use crate::pages::FeedPage;
use crate::pages::IdentityPage;
use crate::pages::PostDetailPage;
use crate::pages::ProfilePage;
use crate::pages::SettingsPage;
use crate::pages::WalletPage;
use crate::platform::Platform;
use crate::state::AppState;
use leptos::prelude::provide_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoView;
use leptos::prelude::RwSignal;
use leptos::prelude::{component, view};
use leptos_meta::provide_meta_context;
use leptos_meta::Title;
use leptos_router::components::Route;
use leptos_router::components::Router;
use leptos_router::components::Routes;
use leptos_router::path;
use thaw::ConfigProvider;
use thaw::Theme;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(AppState::started());
    provide_context(Platform::detected());

    view! {
        <Title text="URMA Wire — Social journalism" />
        <ConfigProvider theme=RwSignal::new(Theme::dark())>
        <Router>
            <div class="app-layout">
                <Navbar />
                <main class="main-content">
                    <div id="network-content">
                    <Routes fallback=|| view! {
                        <Missing heading="Page not found">
                            <p>"The requested route does not exist in URMA Wire."</p>
                        </Missing>
                    }>
                        <Route path=path!("/") view=FeedPage />
                        <Route path=path!("/post/:txid") view=PostDetailPage />
                        <Route path=path!("/profile/:author") view=ProfilePage />
                        <Route path=path!("/compose") view=ComposePage />
                        <Route path=path!("/settings") view=SettingsPage />
                        <Route path=path!("/wallet") view=WalletPage />
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
