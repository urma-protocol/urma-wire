use crate::components::Said;
use crate::data::records;
use crate::data::records::Record;
use crate::state::session::AppState;
use leptos::prelude::expect_context;
use leptos::prelude::Effect;
use leptos::prelude::Get;
use leptos::prelude::RwSignal;
use leptos::prelude::Set;
use leptos::prelude::Signal;
use leptos_router::hooks::use_params_map;
use tracing::warn;
use wire_app_shared::Post;
use wire_app_shared::Txid;

#[derive(Clone)]
pub enum Showing {
    Loading,
    Post(Post),
    Unknown(String),
}

#[derive(Clone, Copy)]
pub struct Thread {
    pub showing: RwSignal<Showing>,
    pub replies: RwSignal<Vec<Post>>,
    pub trouble: Signal<Said>,
}

impl Thread {
    pub fn followed() -> Self {
        let state = expect_context::<AppState>();
        let params = use_params_map();
        let showing = RwSignal::new(Showing::Loading);
        let replies = RwSignal::new(Vec::<Post>::new());
        let trouble = RwSignal::new(Said::Nothing);
        Effect::new(move |_ran_before: Option<()>| {
            let chain = state.active_chain.get();
            let Some(requested) = params.get().get("txid") else {
                showing.set(Showing::Unknown(String::new()));
                return;
            };
            let txid = match Txid::from_display_hex(&requested) {
                Ok(txid) => txid,
                Err(error) => {
                    warn!(error = %error, "post link carries an unreadable txid");
                    showing.set(Showing::Unknown(requested));
                    return;
                }
            };
            showing.set(Showing::Loading);
            replies.set(Vec::new());
            trouble.set(Said::Nothing);
            leptos::task::spawn_local(async move {
                match records::post(chain, txid).await {
                    Ok(Record::Found(post)) => {
                        state.reply_to(post.clone());
                        showing.set(Showing::Post(post));
                    }
                    Ok(Record::Absent) => showing.set(Showing::Unknown(txid.to_display_hex())),
                    Err(failure) => {
                        warn!(failure = %failure, "post record could not be read");
                        showing.set(Showing::Unknown(txid.to_display_hex()));
                        trouble.set(Said::Something(failure.reported()));
                    }
                }
                match records::replies(chain, txid).await {
                    Ok(found) => replies.set(found),
                    Err(failure) => {
                        warn!(failure = %failure, "replies could not be read");
                        trouble.set(Said::Something(failure.reported()));
                    }
                }
            });
        });
        Self {
            showing,
            replies,
            trouble: trouble.into(),
        }
    }
}
