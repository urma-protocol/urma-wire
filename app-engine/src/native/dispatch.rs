use crate::errors::NativeError;
use crate::native::media::fetch;
use crate::native::service::Service;
use wire_app_core::contract::{Request, Response};
use wire_app_core::model::Chain;

impl Service {
    pub fn handle(&self, request: Request) -> Result<Response, NativeError> {
        match request {
            Request::IdentityStatus => Ok(Response::Identity(self.identity_status()?)),
            Request::CreateIdentity { password, replace } => {
                let created = self.create_identity(password, replace)?;
                Ok(Response::Created {
                    status: created.status,
                    phrase: created.words.to_string(),
                })
            }
            Request::RecoverIdentity {
                phrase,
                password,
                replace,
            } => Ok(Response::Identity(
                self.recover_identity(phrase, password, replace)?,
            )),
            Request::ImportIdentity {
                vault,
                password,
                replace,
            } => Ok(Response::Identity(
                self.import_identity(vault, password, replace)?,
            )),
            Request::Unlock(credential) => Ok(Response::Identity(self.unlock(credential)?)),
            Request::Lock => Ok(Response::Identity(self.lock()?)),
            Request::Wallet { chain } => {
                served(chain)?;
                Ok(Response::Wallet(self.wallet()?))
            }
            Request::Feed { chain } => {
                served(chain)?;
                Ok(Response::Feed(self.feed()?))
            }
            Request::StartSync { chain } => {
                served(chain)?;
                Ok(Response::Sync(self.start_sync()?))
            }
            Request::SyncStatus { chain } => {
                served(chain)?;
                Ok(Response::Sync(self.sync_snapshot()))
            }
            Request::Mempool { chain } => {
                served(chain)?;
                Ok(Response::Mempool(self.mempool()?))
            }
            Request::Post { chain, txid } => {
                served(chain)?;
                Ok(Response::Post(self.post(txid)?))
            }
            Request::Replies { chain, txid } => {
                served(chain)?;
                Ok(Response::Replies(self.replies(txid)?))
            }
            Request::Health { chain } => {
                served(chain)?;
                Ok(Response::Health(self.connection_health()?))
            }
            Request::Transport { chain } => {
                served(chain)?;
                Ok(Response::Transport(self.transport()?))
            }
            Request::Quote { chain, content } => {
                served(chain)?;
                Ok(Response::Publication(self.quote(&content)?))
            }
            Request::DiscardQuote { id } => {
                self.discard_quote(&id)?;
                Ok(Response::Discarded)
            }
            Request::PublicationStatus => Ok(Response::Pending(self.status()?)),
            Request::Advance { id, approve } => {
                Ok(Response::Publication(self.advance(&id, approve)?))
            }
            Request::FetchImage { url } => Ok(Response::Image(fetch(&url)?)),
            Request::Source => Ok(Response::Source(self.source_status())),
            Request::AdoptSource(choice) => Ok(Response::SourceAdopted(self.adopt_source(choice)?)),
        }
    }
}

fn served(chain: Chain) -> Result<(), NativeError> {
    match chain {
        Chain::Litecoin => Ok(()),
        Chain::Bitcoin => Err(NativeError::WrongChain),
    }
}
