use crate::components::label_field::walk;
use crate::components::label_field::Offered;
use crate::components::reveal::reveal;
use crate::components::LabelField;
use crate::components::Warning;
use crate::config::METER_NEAR_PERCENT;
use crate::config::REVEAL_DELAY_MS;
use crate::data::publication;
use crate::data::publication::Meant;
use crate::data::publication::Overridden;
use crate::data::topics::Suggestion;
use crate::state::addressing::Speaking;
use crate::state::AppState;
use crate::state::Said;
use leptos::prelude::expect_context;
use leptos::prelude::AriaAttributes;
use leptos::prelude::Callable;
use leptos::prelude::Callback;
use leptos::prelude::ClassAttribute;
use leptos::prelude::CollectView;
use leptos::prelude::Effect;
use leptos::prelude::ElementChild;
use leptos::prelude::Get;
use leptos::prelude::GetUntracked;
use leptos::prelude::GetValue;
use leptos::prelude::GlobalAttributes;
use leptos::prelude::IntoAny;
use leptos::prelude::IntoView;
use leptos::prelude::NodeRef;
use leptos::prelude::NodeRefAttribute;
use leptos::prelude::OnAttribute;
use leptos::prelude::PropAttribute;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Signal;
use leptos::prelude::StoredValue;
use leptos::prelude::{component, event_target, event_target_value, view};
use tracing::warn;
use wire_app_core::composed::accept_at;
use wire_app_core::composed::typing_at;
use wire_app_core::composed::Token;
use wire_app_core::composed::Typed;
use wire_app_core::publication::Addressed;
use wire_app_core::topics::Kind;

#[derive(Clone, PartialEq, Eq)]
pub enum Reading {
    Topics(Meant),
    Unreadable(String),
}

#[component]
pub fn ComposeBox(#[prop(default = Speaking::Freely)] speaking: Speaking) -> impl IntoView {
    let state = expect_context::<AppState>();
    let addressed = speaking.addressed();
    let key = StoredValue::new(speaking.draft_key());
    let label = speaking.publish_label();
    let claiming = speaking.claims_focus();
    let text_label = speaking.text_label();
    let placeholder = speaking.placeholder();
    let wire = RwSignal::new(String::new());
    let hashtags = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let trouble = RwSignal::new(Said::Nothing);
    let text = move || state.draft(&key.get_value());
    let max_bytes = addressed.max_bytes();
    let meant = move || read_topics(&text(), &wire.get(), &hashtags.get());
    let blocked = move || {
        busy.get() || state.following.get() || text().trim().is_empty() || text().len() > max_bytes
    };
    let submit = Callback::new(move |_asked: ()| {
        if blocked() {
            return;
        }
        quote_draft(Quoting {
            state,
            key,
            addressed,
            wire,
            hashtags,
            busy,
            trouble,
        });
    });
    let meter = move || match text().len() {
        len if len > max_bytes => "composer-meter over",
        len if len * 100 >= max_bytes * METER_NEAR_PERCENT => "composer-meter near",
        _ => "composer-meter",
    };
    view! {
        <div class="composer">
            <ReplyContext speaking=speaking />
            <ComposerText key=key text_label=text_label placeholder=placeholder claiming=claiming submit=submit />
            <TopicPreview meant=Signal::derive(meant) />
            <Warning message=trouble.into() />
            <div class="composer-bar">
                <TopicFields wire=wire hashtags=hashtags />
                <span class=meter>
                    {move || text().len()}<span class="composer-meter-max">"/"{max_bytes}</span>
                </span>
                <button class="btn btn-primary btn-sm" title="Ctrl+Enter" disabled=blocked
                    on:click=move |_clicked| submit.run(())>
                    {move || match busy.get() { true => "Quoting…", false => label }}
                </button>
            </div>
        </div>
    }
}

fn read_topics(text: &str, wire: &str, hashtags: &str) -> Reading {
    match Meant::read(text, wire, hashtags) {
        Ok(meant) => Reading::Topics(meant),
        Err(error) => {
            warn!(error = %error, "draft topics could not be read");
            Reading::Unreadable(error.to_string())
        }
    }
}

#[derive(Clone, Copy)]
struct Quoting {
    state: AppState,
    key: StoredValue<String>,
    addressed: Addressed,
    wire: RwSignal<String>,
    hashtags: RwSignal<String>,
    busy: RwSignal<bool>,
    trouble: RwSignal<Said>,
}

fn quote_draft(quoting: Quoting) {
    let Quoting {
        state,
        key,
        addressed,
        wire,
        hashtags,
        busy,
        trouble,
    } = quoting;
    trouble.set(Said::Nothing);
    let text = state.draft(&key.get_value());
    let meant = match read_topics(&text, &wire.get_untracked(), &hashtags.get_untracked()) {
        Reading::Topics(meant) => meant,
        Reading::Unreadable(reason) => {
            trouble.set(Said::Something(reason));
            return;
        }
    };
    let spoken_wire = meant.wire.clone();
    let spoken_tags = meant.hashtags.clone();
    let content = meant.spoken(text, addressed);
    busy.set(true);
    let chain = state.active_chain.get_untracked();
    leptos::task::spawn_local(async move {
        let quoted = publication::quote(chain, content).await;
        busy.set(false);
        let quote = match quoted {
            Ok(quote) => quote,
            Err(fault) => {
                warn!(fault = %fault, "publication quote refused");
                trouble.set(Said::Something(fault.message));
                return;
            }
        };
        state.remember_topics(&spoken_wire, &spoken_tags);
        state.clear_draft(&key.get_value());
        wire.set(String::new());
        hashtags.set(String::new());
        state.publication_error.set(Said::Nothing);
        state.publication.set(Some(quote));
        match addressed {
            Addressed::Thread(_) => state.stop_replying(),
            Addressed::Standalone => (),
        }
    });
}

#[component]
fn TopicFields(wire: RwSignal<String>, hashtags: RwSignal<String>) -> impl IntoView {
    view! {
        <details class="composer-topics">
            <summary aria-label="Wire and hashtags">
                <span class="composer-topics-mark" aria-hidden="true">"#"</span>
                <span class="composer-topics-text">{move || topic_summary(&wire.get(), &hashtags.get())}</span>
            </summary>
            <div class="composer-fields">
                <p class="composer-fields-note">
                    "Type /wire and #hashtag in the text, or add them here. Text wins on conflict."
                </p>
                <LabelField label="Wire" placeholder="rosint" kind=Kind::Wire value=wire single=true />
                <LabelField label="Hashtags" placeholder="investigation documents" kind=Kind::Hashtag value=hashtags single=false />
            </div>
        </details>
    }
}

#[derive(Clone, Copy)]
struct Cursor {
    state: AppState,
    key: StoredValue<String>,
    caret: RwSignal<usize>,
    area: NodeRef<leptos::html::Textarea>,
    highlighted: RwSignal<usize>,
    open: RwSignal<bool>,
}

impl Cursor {
    fn text(&self) -> String {
        self.state.draft(&self.key.get_value())
    }

    fn typing(&self) -> Typed {
        match typing_at(&self.text(), self.caret.get()) {
            Ok(typed) => typed,
            Err(error) => {
                warn!(error = %error, "caret position could not be read");
                Typed::Plain
            }
        }
    }

    fn offered(&self) -> Vec<Suggestion> {
        match self.typing() {
            Typed::Token(token) => self.state.topics.get().suggestions(
                token.kind,
                &token.label,
                &self.state.seen_topics(),
            ),
            Typed::Plain => Vec::new(),
        }
    }

    fn accept(&self, suggestion: Suggestion) {
        let Typed::Token(token) = self.typing() else {
            return;
        };
        place(Placing {
            state: self.state,
            key: self.key,
            area: self.area,
            caret: self.caret,
            token,
            text: self.text(),
            label: suggestion.label,
        });
        self.highlighted.set(0);
        self.open.set(false);
    }

    fn track(&self, event: &leptos::ev::Event) {
        let target = event_target::<web_sys::HtmlTextAreaElement>(event);
        match selection_of(&target) {
            Selection::At(at) => self.caret.set(at),
            Selection::Unknown => (),
        }
    }

    fn typed(&self, event: &leptos::ev::Event) {
        self.state
            .set_draft(&self.key.get_value(), event_target_value(event));
        self.track(event);
        self.open.set(true);
        self.highlighted.set(0);
    }

    fn claim(&self) {
        let Some(area) = self.area.get_untracked() else {
            return;
        };
        match area.focus() {
            Ok(()) => (),
            Err(error) => warn!(error = ?error, "reply composer could not take focus"),
        }
        let held = self.state.draft_untracked(&self.key.get_value());
        let at = match u32::try_from(held.len()) {
            Ok(at) => at,
            Err(error) => {
                warn!(error = %error, "draft length does not fit the textarea");
                return;
            }
        };
        match area.set_selection_range(at, at) {
            Ok(()) => self.caret.set(held.len()),
            Err(error) => warn!(error = ?error, "caret could not be placed in the reply draft"),
        }
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(REVEAL_DELAY_MS).await;
            reveal(&area);
        });
    }

    fn keyed(&self, event: &leptos::ev::KeyboardEvent, submit: Callback<()>) {
        if (event.ctrl_key() || event.meta_key()) && event.key() == "Enter" {
            event.prevent_default();
            submit.run(());
            return;
        }
        let cursor = *self;
        walk(
            event,
            self.offered(),
            self.highlighted,
            self.open,
            move |chosen| cursor.accept(chosen),
        );
    }
}

#[component]
fn ComposerText(
    key: StoredValue<String>,
    text_label: &'static str,
    placeholder: &'static str,
    claiming: bool,
    submit: Callback<()>,
) -> impl IntoView {
    let cursor = Cursor {
        state: expect_context::<AppState>(),
        key,
        caret: RwSignal::new(0usize),
        area: NodeRef::new(),
        highlighted: RwSignal::new(0usize),
        open: RwSignal::new(false),
    };
    if claiming {
        Effect::new(move |_ran_before: Option<()>| cursor.claim());
    }
    view! {
        <div class="composer-text-wrap">
            <textarea node_ref=cursor.area class="composer-text" rows="3" autocomplete="off"
                aria-label=text_label
                placeholder=placeholder
                prop:value=move || cursor.text()
                on:input=move |event| cursor.typed(&event)
                on:click=move |event| cursor.track(&event)
                on:keyup=move |event| cursor.track(&event)
                on:blur=move |_left| cursor.open.set(false)
                on:keydown=move |event| cursor.keyed(&event, submit) />
            {move || {
                let ranked = cursor.offered();
                if !cursor.open.get() || ranked.is_empty() {
                    return ().into_any();
                }
                view! {
                    <Offered ranked=ranked highlighted=cursor.highlighted
                        accept=Callback::new(move |chosen| cursor.accept(chosen)) />
                }.into_any()
            }}
        </div>
    }
}

enum Selection {
    At(usize),
    Unknown,
}

fn selection_of(target: &web_sys::HtmlTextAreaElement) -> Selection {
    let start = match target.selection_start() {
        Ok(start) => start,
        Err(error) => {
            warn!(error = ?error, "caret position is unavailable");
            return Selection::Unknown;
        }
    };
    let Some(start) = start else {
        return Selection::Unknown;
    };
    match usize::try_from(start) {
        Ok(caret) => Selection::At(caret),
        Err(error) => {
            warn!(error = %error, "caret position does not fit an offset");
            Selection::Unknown
        }
    }
}

struct Placing {
    state: AppState,
    key: StoredValue<String>,
    area: NodeRef<leptos::html::Textarea>,
    caret: RwSignal<usize>,
    token: Token,
    text: String,
    label: String,
}

fn place(placing: Placing) {
    let Placing {
        state,
        key,
        area,
        caret,
        token,
        text,
        label,
    } = placing;
    let accepted = match accept_at(&text, &token, &label) {
        Ok(accepted) => accepted,
        Err(error) => {
            warn!(error = %error, "suggestion could not be placed in the draft");
            return;
        }
    };
    state.set_draft(&key.get_value(), accepted.text);
    caret.set(accepted.caret);
    let Some(area) = area.get_untracked() else {
        return;
    };
    let at = match u32::try_from(accepted.caret) {
        Ok(at) => at,
        Err(error) => {
            warn!(error = %error, "caret offset does not fit the textarea");
            return;
        }
    };
    match area.set_selection_range(at, at) {
        Ok(()) => (),
        Err(error) => warn!(error = ?error, "caret could not be placed after the label"),
    }
    match area.focus() {
        Ok(()) => (),
        Err(error) => warn!(error = ?error, "composer could not take focus back"),
    }
}

#[component]
fn TopicPreview(meant: Signal<Reading>) -> impl IntoView {
    view! {
        {move || match meant.get() {
            Reading::Unreadable(reason) => view! {
                <p class="alert alert-error" role="alert">{reason}</p>
            }.into_any(),
            Reading::Topics(meant) => {
                if meant.wire.is_empty() && meant.hashtags.is_empty() && !meant.conflicted() {
                    return ().into_any();
                }
                view! {
                    <div class="composer-meta" role="status">
                        <span class="composer-meta-label">"Will publish as"</span>
                        <span class="composer-meta-chips">
                            <Spelled meant=meant.clone() />
                        </span>
                        <Conflicts meant=meant />
                    </div>
                }.into_any()
            }
        }}
    }
}

#[component]
fn Spelled(meant: Meant) -> impl IntoView {
    let mut spelled: Vec<String> = Vec::new();
    if !meant.wire.is_empty() {
        spelled.push(format!("/{}", meant.wire));
    }
    for tag in &meant.hashtags {
        spelled.push(format!("#{tag}"));
    }
    if spelled.is_empty() {
        return view! { <span class="composer-meta-none">"no topics"</span> }.into_any();
    }
    spelled
        .into_iter()
        .map(|label| view! { <span class="composer-meta-chip">{label}</span> })
        .collect_view()
        .into_any()
}

#[component]
fn Conflicts(meant: Meant) -> impl IntoView {
    let mut notes: Vec<String> = Vec::new();
    match &meant.overridden_wire {
        Overridden::Field(overridden) => notes.push(format!(
            "The text names /{}, so the wire field /{overridden} is not applied.",
            meant.wire
        )),
        Overridden::Nothing => (),
    }
    if !meant.extra_wires.is_empty() {
        let spelled: Vec<String> = meant
            .extra_wires
            .iter()
            .map(|wire| format!("/{wire}"))
            .collect();
        notes.push(format!(
            "A record carries one wire, so {} stays plain text.",
            spelled.join(", ")
        ));
    }
    if !meant.dropped_hashtags.is_empty() {
        let spelled: Vec<String> = meant
            .dropped_hashtags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect();
        notes.push(format!(
            "At most eight hashtags are carried, so {} stays plain text.",
            spelled.join(", ")
        ));
    }
    if notes.is_empty() {
        return ().into_any();
    }
    view! {
        <ul class="composer-meta-notes">
            {notes.into_iter().map(|note| view! { <li>{note}</li> }).collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn ReplyContext(speaking: Speaking) -> impl IntoView {
    let state = expect_context::<AppState>();
    let Speaking::InReplyTo(post) = speaking else {
        return ().into_any();
    };
    let author = post.author;
    let excerpt = excerpt_of(&post.text);
    view! {
        <div class="composer-reply">
            <span class="composer-reply-mark" aria-hidden="true">"↳"</span>
            <span class="composer-reply-text">
                "Replying to "<strong>{move || state.display_name(author)}</strong>
                <span class="composer-reply-excerpt">{excerpt}</span>
            </span>
            <button type="button" class="composer-reply-cancel" aria-label="Cancel reply" title="Cancel reply"
                on:click=move |_clicked| state.stop_replying()>"✕"</button>
        </div>
    }
    .into_any()
}

fn excerpt_of(text: &str) -> String {
    let flattened: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut excerpt = String::new();
    for character in flattened.chars() {
        if excerpt.chars().count() >= 64 {
            excerpt.push('…');
            break;
        }
        excerpt.push(character);
    }
    excerpt
}

fn topic_summary(wire: &str, hashtags: &str) -> String {
    let wire = wire.trim().trim_start_matches('/');
    let tags = hashtags.split_whitespace().count();
    if wire.is_empty() && tags == 0 {
        return "Wire & hashtags".to_owned();
    }
    if wire.is_empty() {
        return format!("{tags} hashtags");
    }
    if tags == 0 {
        return format!("/{wire}");
    }
    format!("/{wire} · {tags} hashtags")
}
