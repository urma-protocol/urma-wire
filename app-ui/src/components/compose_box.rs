use crate::components::label_field::walk;
use crate::components::label_field::Offered;
use crate::components::reveal::reveal;
use crate::components::LabelField;
use crate::components::markdown::PostText;
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
use leptos::prelude::Update;
use leptos::prelude::{component, event_target, event_target_value, view};
use tracing::warn;
use wire_app_core::composed::accept_at;
use wire_app_core::composed::typing_at;
use wire_app_core::composed::Token;
use wire_app_core::composed::Typed;
use wire_app_core::composer::{compose, Photo};
use wire_app_core::config::MAX_IMAGES_PER_POST;
use wire_app_core::errors::FaultKind;
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
    let (held_wire, held_hashtags) = state.draft_labels(&key.get_value());
    let wire = RwSignal::new(held_wire);
    let hashtags = RwSignal::new(held_hashtags);
    let labels_revision = RwSignal::new(state.draft_clear_revision(&key.get_value()));
    Effect::new(move |_previous: Option<()>| {
        if labels_revision.get() == state.draft_clear_revision(&key.get_value()) {
            state.set_draft_labels(&key.get_value(), wire.get(), hashtags.get());
        }
    });
    Effect::new(move |previous: Option<u64>| {
        let revision = state.draft_clear_revision(&key.get_value());
        if previous.is_some_and(|before| before != revision) {
            wire.set(String::new());
            hashtags.set(String::new());
            labels_revision.set(revision);
        }
        revision
    });
    let busy = RwSignal::new(false);
    let trouble = RwSignal::new(Said::Nothing);
    let unfinished = RwSignal::new(false);
    let text = move || state.draft(&key.get_value());
    let photos = move || state.photos(&key.get_value());
    let max_bytes = addressed.max_bytes();
    let meant = move || read_topics(&text(), &wire.get(), &hashtags.get());
    let blocked = move || busy.get() || (text().trim().is_empty() && photos().is_empty());
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
            unfinished,
        });
    });
    let bytes = move || compose(&text(), &photos(), usize::MAX, usize::MAX).map(|ready| ready.len()).unwrap_or(text().len());
    let meter = move || match bytes() {
        len if len > max_bytes => "composer-meter over",
        len if len * 100 >= max_bytes * METER_NEAR_PERCENT => "composer-meter near",
        _ => "composer-meter",
    };
    view! {
        <div class="composer">
            <ReplyContext speaking=speaking />
            <ComposerText key=key text_label=text_label placeholder=placeholder claiming=claiming submit=submit />
            <PhotoComposer key=key max_bytes=max_bytes />
            <TopicPreview meant=Signal::derive(meant) />
            <Warning message=trouble.into() />
            <PanelLink unfinished=unfinished />
            <div class="composer-bar">
                <TopicFields wire=wire hashtags=hashtags />
                <span class=meter>
                    {bytes}<span class="composer-meter-max">"/"{max_bytes}</span>
                </span>
                <button class="btn btn-primary btn-sm" title="Ctrl+Enter" disabled=blocked
                    on:click=move |_clicked| submit.run(())>
                    {move || match busy.get() { true => "Quoting…", false => label }}
                </button>
            </div>
        </div>
    }
}

#[component]
fn PanelLink(unfinished: RwSignal<bool>) -> impl IntoView {
    view! {
        {move || match unfinished.get() {
            true => view! {
                <p class="action-desc"><a href="/compose">"Open your publication to resume or abandon it"</a></p>
            }.into_any(),
            false => ().into_any(),
        }}
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
    unfinished: RwSignal<bool>,
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
        unfinished,
    } = quoting;
    trouble.set(Said::Nothing);
    unfinished.set(false);
    let draft_key = key.get_value();
    let revision = state.draft_version(&draft_key);
    let manual_text = state.draft(&draft_key);
    let text = match compose(
        &manual_text,
        &state.photos(&draft_key),
        addressed.max_bytes(),
        MAX_IMAGES_PER_POST,
    ) {
        Ok(text) => text,
        Err(error) => {
            trouble.set(Said::Something(error.to_string()));
            return;
        }
    };
    let meant = match read_topics(&manual_text, &wire.get_untracked(), &hashtags.get_untracked()) {
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
                unfinished.set(fault.kind == FaultKind::Publication);
                trouble.set(Said::Something(fault.message));
                return;
            }
        };
        if state.draft_version(&draft_key) != revision {
            state.invalidated_reviews.update(|old| { old.insert(quote.id.clone()); });
            if let Err(fault) = publication::discard(quote.id).await {
                warn!(fault = %fault, "outdated quote could not be discarded");
                trouble.set(Said::Something(fault.message));
            }
            return;
        }
        state.remember_topics(&spoken_wire, &spoken_tags);
        state.accept_review(&quote.id);
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

#[component]
fn PhotoComposer(key: StoredValue<String>, max_bytes: usize) -> impl IntoView {
    let state = expect_context::<AppState>();
    let url_id = StoredValue::new(format!("composer-photo-url-{}", key.get_value()));
    let alt_id = StoredValue::new(format!("composer-photo-alt-{}", key.get_value()));
    let url_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let open = RwSignal::new(false);
    let editing = RwSignal::new(None::<usize>);
    let url = RwSignal::new(String::new());
    let alt = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    Effect::new(move |_previous: Option<()>| {
        if open.get() {
            if let Some(input) = url_input.get() {
                if let Err(reason) = input.focus() {
                    warn!(error = ?reason, "photo URL input could not take focus");
                }
            }
        }
    });
    let save = move |_clicked: leptos::ev::MouseEvent| {
        let photo = match Photo::new(&url.get_untracked(), &alt.get_untracked()) {
            Ok(photo) => photo,
            Err(reason) => {
                error.set(Some(reason.to_string()));
                return;
            }
        };
        let held_key = key.get_value();
        let mut photos = state.photos(&held_key);
        match editing.get_untracked() {
            Some(index) if index < photos.len() => photos[index] = photo,
            _ => photos.push(photo),
        }
        if let Err(reason) = compose(&state.draft(&held_key), &photos, max_bytes, MAX_IMAGES_PER_POST) {
            error.set(Some(reason.to_string()));
            return;
        }
        state.set_photos(&held_key, photos);
        error.set(None);
        editing.set(None);
        open.set(false);
    };
    view! {
        <div class="composer-photos">
            <button type="button" class="btn btn-secondary composer-photo-action"
                on:click=move |_clicked| {
                    editing.set(None);
                    url.set(String::new());
                    alt.set(String::new());
                    error.set(None);
                    open.set(true);
                }>"Add photo"</button>
            <span class="composer-photo-count">{move || format!("{} / {} photos", state.photos(&key.get_value()).len(), MAX_IMAGES_PER_POST)}</span>
            {move || state.photos(&key.get_value()).into_iter().enumerate().map(|(index, photo)| {
                let label = photo.alt.clone();
                let address = photo.url.clone();
                let preview = photo.markdown();
                view! {
                    <div class="composer-photo-card">
                        <div class="composer-photo-info"><strong>{label.clone()}</strong><span>{address.clone()}</span></div>
                        <div class="composer-photo-preview"><PostText text=preview wire=String::new() hashtags=Vec::new() /></div>
                        <div class="composer-photo-actions">
                            <button type="button" class="btn btn-secondary composer-photo-action" aria-label=format!("Edit photo: {label}")
                                on:click=move |_clicked| {
                                    editing.set(Some(index));
                                    url.set(address.clone());
                                    alt.set(label.clone());
                                    error.set(None);
                                    open.set(true);
                                }>"Edit"</button>
                            <button type="button" class="btn btn-secondary composer-photo-action" aria-label="Remove photo"
                                on:click=move |_clicked| {
                                    let held_key = key.get_value();
                                    let mut photos = state.photos(&held_key);
                                    if index < photos.len() {
                                        photos.remove(index);
                                        state.set_photos(&held_key, photos);
                                    }
                                    open.set(false);
                                    error.set(None);
                                }>"Remove"</button>
                        </div>
                    </div>
                }
            }).collect_view()}
            {move || match open.get() {
                false => ().into_any(),
                true => view! {
                    <div class="composer-photo-form">
                        <label for=url_id.get_value()>"HTTPS image URL"</label>
                        <input id=url_id.get_value() node_ref=url_input type="url" inputmode="url" autocomplete="url"
                            placeholder="https://cdn.example/photo.jpg" prop:value=move || url.get()
                            on:input=move |event| url.set(event_target_value(&event)) />
                        <label for=alt_id.get_value()>"Alt text"</label>
                        <input id=alt_id.get_value() type="text" autocomplete="off"
                            placeholder="Describe the photo" prop:value=move || alt.get()
                            on:input=move |event| alt.set(event_target_value(&event)) />
                        <p class="composer-photo-note">"Paste a direct HTTPS image link. Images stay on their external host; loading follows your media setting."</p>
                        {move || error.get().map(|message| view! { <p class="alert alert-error" role="alert">{message}</p> })}
                        <div class="composer-photo-actions">
                            <button type="button" class="btn btn-primary composer-photo-action" on:click=save>
                                {move || match editing.get() { Some(_) => "Save photo", None => "Add photo" }}
                            </button>
                            <button type="button" class="btn btn-secondary composer-photo-action"
                                on:click=move |_clicked| { open.set(false); error.set(None); }>"Cancel"</button>
                        </div>
                    </div>
                }.into_any(),
            }}
        </div>
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
