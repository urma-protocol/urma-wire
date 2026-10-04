use crate::errors::NativeError;
use crate::native::media::fetch;
use crate::native::service::Service;
use wire_app_core::contract::{Request, Response};
use wire_app_core::model::Chain;

impl Service {
    pub fn handle(&self, request: Request) -> Result<Response, NativeError> {
        match request {
            Request::IdentityStatus => Ok(Response::Identity(self.identity_status()?)),
            Request::ImportIdentity { vault, password } => {
                Ok(Response::Identity(self.import_identity(vault, password)?))
            }
            Request::Unlock(credential) => {
                self.unlock(credential)?;
                Ok(Response::Unlocked)
            }
            Request::Lock => {
                self.lock();
                Ok(Response::Locked)
            }
            Request::Wallet { chain } => {
                served(chain)?;
                Ok(Response::Wallet(self.wallet()?))
            }
            Request::Feed { chain } => {
                served(chain)?;
                Ok(Response::Feed(self.sync()?))
            }
            Request::Sync { chain, max_blocks } => {
                served(chain)?;
                Ok(Response::Sync(self.sync_slice(max_blocks)?))
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
