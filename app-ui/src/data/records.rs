use crate::data::engine::ask;
use crate::data::failure::Failure;
use serde_json::json;
use wire_app_shared::live_model::Feed;
use wire_app_shared::Chain;
use wire_app_shared::Post;
use wire_app_shared::Txid;

pub enum Record {
    Found(Post),
    Absent,
}

pub async fn feed(chain: Chain) -> Result<Feed, Failure> {
    ask("get_feed", &json!({ "chain": chain.code() })).await
}

pub async fn post(chain: Chain, txid: Txid) -> Result<Record, Failure> {
    let found: Option<Post> = ask(
        "get_post",
        &json!({ "chain": chain.code(), "txid": txid.to_display_hex() }),
    )
    .await?;
    let Some(post) = found else {
        return Ok(Record::Absent);
    };
    Ok(Record::Found(post))
}

pub async fn replies(chain: Chain, txid: Txid) -> Result<Vec<Post>, Failure> {
    ask(
        "get_replies",
        &json!({ "chain": chain.code(), "txid": txid.to_display_hex() }),
    )
    .await
}
