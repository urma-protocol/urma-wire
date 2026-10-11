use crate::config::{PULSE_FRAME_MS, PULSE_SWEEP_MS};
use crate::state::AppState;
use crate::state::Synced;
use leptos::prelude::expect_context;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CustomAttribute;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::Memo;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::{component, view};
use wire_app_core::connection::{LocalStatus, SourceKind, SourceStanding, TransportStatus};

pub struct Trace;

impl Trace {
    pub const ENTRY: (f64, f64) = (54.0, 60.0);
    pub const EXIT: (f64, f64) = (458.0, 60.0);
    pub const BOTTOM: (f64, f64) = (256.0, 386.0);
    const CURVE_STEPS: u8 = 48;
    const LEFT_ARC: [(f64, f64); 4] = [(120.0, 277.0), (120.0, 343.0), (175.0, 386.0), Self::BOTTOM];
    const RIGHT_ARC: [(f64, f64); 4] = [Self::BOTTOM, (337.0, 386.0), (392.0, 343.0), (392.0, 277.0)];
    pub const CUTS: [(f64, &'static str); 4] = [
        (226.0, "M209 406L218 396 215 390 227 380 223 373 244 359"),
        (245.0, "M246 357L242 370 248 381 243 393 247 412"),
        (267.0, "M266 357L270 370 264 381 269 393 265 412"),
        (286.0, "M303 406L294 396 297 390 285 380 289 373 268 359"),
    ];
    const BREAK_STAR: &'static str = "256,345 259,350 263,348 261,352 266,355 261,358 263,362 259,360 256,365 253,360 249,362 251,358 246,355 251,352 249,348 253,350";

    fn points() -> Vec<(f64, f64)> {
        let mut points = vec![Self::ENTRY, (120.0, 60.0), (120.0, 277.0)];
        for arc in [Self::LEFT_ARC, Self::RIGHT_ARC] {
            for step in 1..=Self::CURVE_STEPS {
                points.push(cubic(arc, f64::from(step) / f64::from(Self::CURVE_STEPS)));
            }
        }
        points.push((392.0, 60.0));
        points.push(Self::EXIT);
        points
    }
}

fn cubic(arc: [(f64, f64); 4], t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    arc.iter()
        .zip(weights)
        .fold((0.0, 0.0), |(x, y), (point, w)| (x + w * point.0, y + w * point.1))
}

pub fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

pub fn pulse_at(fraction: f64) -> (f64, f64) {
    let points = Trace::points();
    let total: f64 = points.windows(2).map(|w| distance(w[0], w[1])).sum();
    let mut remaining = fraction.clamp(0.0, 1.0) * total;
    for pair in points.windows(2) {
        let length = distance(pair[0], pair[1]);
        if remaining <= length {
            let t = if length == 0.0 { 0.0 } else { remaining / length };
            return (
                pair[0].0 + (pair[1].0 - pair[0].0) * t,
                pair[0].1 + (pair[1].1 - pair[0].1) * t,
            );
        }
        remaining -= length;
    }
    Trace::EXIT
}

fn whole(n: u64) -> f64 {
    n.to_be_bytes()
        .iter()
        .fold(0.0, |acc, byte| acc * 256.0 + f64::from(*byte))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mark {
    pub fraction: f64,
    pub pulse_shown: bool,
    pub healed: bool,
}

impl Mark {
    pub fn of(synced: &Synced) -> Self {
        match synced {
            Synced::Unchecked => Self { fraction: 0.0, pulse_shown: false, healed: false },
            Synced::Connecting => Self { fraction: 0.0, pulse_shown: true, healed: false },
            Synced::Verifying { done, total } | Synced::Retrying { done, total, .. } => Self {
                fraction: if *total == 0 { 1.0 } else { (whole(*done) / whole(*total)).min(1.0) },
                pulse_shown: true,
                healed: false,
            },
            Synced::Building => Self { fraction: 1.0, pulse_shown: true, healed: false },
            Synced::Indexed { complete, .. } => Self { fraction: 1.0, pulse_shown: !complete, healed: *complete },
            Synced::Failed(_reason) => Self { fraction: 0.0, pulse_shown: false, healed: false },
        }
    }

    pub fn pulse(&self) -> (f64, f64) {
        pulse_at(self.fraction)
    }

    pub fn cut_open(&self, seal_x: f64) -> bool {
        !self.healed && self.pulse().0 < seal_x
    }
}

pub fn swept(shown: f64, goal: f64) -> f64 {
    let step = f64::from(PULSE_FRAME_MS) / f64::from(PULSE_SWEEP_MS);
    if goal < shown {
        return goal;
    }
    (shown + step).min(goal)
}

async fn sweep(mark: Memo<Mark>, shown: RwSignal<f64>) {
    loop {
        let goal = mark.get_untracked().fraction;
        let current = shown.get_untracked();
        if (goal - current).abs() > f64::EPSILON {
            shown.set(swept(current, goal));
        }
        gloo_timers::future::TimeoutFuture::new(PULSE_FRAME_MS).await;
    }
}

#[component]
pub fn BrandMark() -> impl IntoView {
    let state = expect_context::<AppState>();
    let mark = Memo::new(move |_previous: Option<&Mark>| Mark::of(&state.synced.get()));
    let shown = RwSignal::new(0.0_f64);
    leptos::task::spawn_local(sweep(mark, shown));
    let pulse = move || pulse_at(shown.get());
    let arrived = move || (mark.get().fraction - shown.get()).abs() <= f64::EPSILON;
    let visible = move || mark.get().pulse_shown || !arrived();
    view! {
        <svg class="brand-mark" viewBox="0 0 512 512" aria-hidden="true">
            <defs>
                <mask id="brand-u-nodes">
                    <rect width="512" height="512" fill="white"/>
                    <g fill="black">
                        <circle cx="132" cy="136" r="4.5"/>
                        <circle cx="131" cy="169" r="4.5"/>
                        <circle cx="135" cy="201" r="4.5"/>
                        <circle cx="380" cy="136" r="4.5"/>
                        <circle cx="381" cy="169" r="4.5"/>
                        <circle cx="377" cy="201" r="4.5"/>
                    </g>
                </mask>
                <mask id="brand-cuts">
                    <rect width="512" height="512" fill="white"/>
                    {Trace::CUTS.iter().map(|(seal_x, d)| {
                        let seal_x = *seal_x;
                        view! {
                            <path class="brand-cut" d=*d
                                opacity=move || match !mark.get().healed && pulse().0 < seal_x { true => "1", false => "0" }/>
                        }
                    }).collect::<Vec<_>>()}
                </mask>
            </defs>
            <rect width="512" height="512" fill="#000"/>
            <g class="brand-frame">
                <path d="M82 141V35h116"/>
                <path d="M314 35h116v106"/>
                <path d="M82 311v112h116"/>
                <path d="M314 423h116V311"/>
            </g>
            <g mask="url(#brand-cuts)">
                <g class="brand-rail" mask="url(#brand-u-nodes)">
                    <path d="M108 60V277C108 352 169 400 256 400C343 400 404 352 404 277V60"/>
                    <path d="M132 60V277C132 333 181 373 256 373C331 373 380 333 380 277V60"/>
                    <path d="M108 60H132M380 60H404"/>
                </g>
                <image href="/assets/circuit-eagle.svg" x="122" y="127" width="268" height="221" attr:preserveAspectRatio="none"/>
            </g>
            <g class="brand-entry">
                <path d="M54 60H132"/>
                <path d="M380 60H458"/>
                <circle cx="54" cy="60" r="5"/>
                <circle cx="458" cy="60" r="5"/>
            </g>
            <circle class="brand-pulse" r="10"
                cx=move || format!("{:.1}", pulse().0)
                cy=move || format!("{:.1}", pulse().1)
                opacity=move || match visible() { true => "1", false => "0" }/>
            {move || match mark.get().healed && arrived() {
                true => view! { <circle class="brand-point healed" cx="256" cy="355" r="8"/> }.into_any(),
                false => view! { <polygon class="brand-point" points=Trace::BREAK_STAR/> }.into_any(),
            }}
        </svg>
    }
}

#[component]
pub fn SyncStatus() -> impl IntoView {
    let state = expect_context::<AppState>();
    let tone = move || match state.synced.get() {
        Synced::Indexed { complete: true } => "brand-sync healed",
        Synced::Retrying { .. } => "brand-sync retrying",
        Synced::Failed(_reason) => "brand-sync failed",
        Synced::Unchecked
        | Synced::Connecting
        | Synced::Verifying { .. }
        | Synced::Building
        | Synced::Indexed { complete: false } => "brand-sync",
    };
    let count = move || match state.synced.get() {
        Synced::Verifying { done, total } | Synced::Retrying { done, total, .. } => {
            view! { <span class="brand-sync-count">{done}"/"{total}</span> }.into_any()
        }
        Synced::Unchecked
        | Synced::Connecting
        | Synced::Building
        | Synced::Indexed { .. }
        | Synced::Failed(_) => ().into_any(),
    };
    let hint = move || match state.synced.get() {
        Synced::Retrying { attempt, reason, .. } => format!("Retry {attempt}: {reason}"),
        Synced::Failed(reason) => reason,
        Synced::Unchecked
        | Synced::Connecting
        | Synced::Verifying { .. }
        | Synced::Building
        | Synced::Indexed { .. } => String::new(),
    };
    view! {
        <div class=tone role="status" title=hint>
            {count}
            <span class="brand-sync-label">{move || state.synced.get().label()}</span>
        </div>
    }
}

pub fn peers_hint(status: &TransportStatus) -> String {
    let warming = status
        .sources
        .iter()
        .any(|source| source.kind == SourceKind::Peers && source.standing == SourceStanding::Cold);
    match (status.peers, warming) {
        (0, true) => "Light client warming up: no Litecoin testnet peers yet".to_owned(),
        (0, false) => "No peers: the light client has no Litecoin testnet node to ask for blocks".to_owned(),
        (peers, _warming) => format!(
            "{peers} peers: Litecoin testnet nodes serving blocks to the built-in light client; headers and proof of work are verified locally"
        ),
    }
}

pub fn sources_hint(status: &TransportStatus) -> String {
    let mut lines = vec!["Public sources, asked when peers cannot serve a block:".to_owned()];
    for source in status.remote_sources() {
        let kind = match source.kind {
            SourceKind::LocalNode => "local node",
            SourceKind::Electrum => "Electrum",
            SourceKind::Http => "HTTP API",
            SourceKind::Peers => "peers",
        };
        let standing = match &source.standing {
            SourceStanding::Ready => "ready".to_owned(),
            SourceStanding::Cold => "cold".to_owned(),
            SourceStanding::Blocked { seconds } => format!("cooling down {seconds} s"),
        };
        lines.push(format!("{} · {kind} · {standing}", source.label));
    }
    lines.join("\n")
}

pub fn local_hint(status: &TransportStatus) -> String {
    match status.local_source() {
        LocalStatus::Configured(source) => match source.standing {
            SourceStanding::Ready => format!("{}: verified local node, preferred for block reads", source.label),
            SourceStanding::Cold => format!("{}: unavailable, unverified or still syncing; checked automatically", source.label),
            SourceStanding::Blocked { seconds } => format!("{}: retrying in {seconds} s; public fallback available", source.label),
        },
        LocalStatus::Unconfigured => "Local node not configured: set rpc_url and node_auth_file in URMA config".into(),
    }
}

fn local_ready(status: &TransportStatus) -> bool {
    match status.local_source() {
        LocalStatus::Configured(source) => source.standing == SourceStanding::Ready,
        LocalStatus::Unconfigured => false,
    }
}

fn net_tone(live: bool, partial: bool) -> &'static str {
    match (live, partial) {
        (false, _partial) => "brand-net-item down",
        (true, true) => "brand-net-item partial",
        (true, false) => "brand-net-item live",
    }
}

#[component]
pub fn Transport() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        {move || state.transport.get().map(|status| {
            let ready = status.ready_sources();
            let total = status.remote_sources().len();
            view! {
                <div class="brand-net">
                    <span role="img" class=net_tone(local_ready(&status), false)
                        title=local_hint(&status) attr:aria-label=local_hint(&status)>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="13" rx="2"/><path d="M8 21h8M12 16v5M7 7h2M7 11h2"/></svg>
                        <span>"Local"</span>
                    </span>
                    <span class=net_tone(status.peers > 0, false) title=peers_hint(&status)>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="8" r="4"/><path d="M4 21v-2a8 8 0 0 1 16 0v2"/></svg>
                        <span>{status.peers}</span>
                    </span>
                    <span class=net_tone(ready > 0, ready < total) title=sources_hint(&status)>
                        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/></svg>
                        <span>{ready}"/"{total}</span>
                    </span>
                </div>
            }
        })}
    }
}
