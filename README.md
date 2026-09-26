# URMA Wire V0

Tauri 2 desktop shell with a Leptos/WASM frontend. The current end-to-end
milestone is **public text posts and replies on Litecoin Testnet**. The protocol
remains URMA V0, including the structured
wire/hashtag extension. Package versions are separate and recorded in the
[crate manifests](app-engine/Cargo.toml). The committed workspace patch table
resolves URMA crates from the neighboring `../urma` checkout for both native
and browser builds.

## Run locally

Feed and Publish reuse the same compact composer and session post draft.
Use **Add photo** to paste an HTTPS image URL and describe it with alt text;
added photos can be previewed, edited and removed before review. The composer
keeps manually entered text and added photos while moving between Feed and
Publish. Editing a draft invalidates its unapproved review; review again to
approve the changed content. Photos remain on their external hosts. Wire does
not upload, copy or preserve the image bytes in a published record.
Wire/hashtag fields expand on demand; exact quote approval remains required.
Feed refreshes confirmed posts on entry. Completed publication panels disappear;
the Publish page retains its additional explanatory information.

With Rust's `wasm32-unknown-unknown` target, Trunk, Cargo Tauri and the platform's
Tauri native dependencies already installed:

```sh
NO_COLOR=true cargo tauri dev
```

Keep the `urma-wire-app` and `urma` repositories beside each other. The sibling
core checkout must include the Litecoin decoder changes (commit `f1547e9` or a
later compatible revision). Cargo, Trunk and Tauri use the committed patches
without a private `.cargo/config.toml` or extra command-line arguments. Separate
app worktrees need the same sibling layout or explicit Cargo path overrides.
The running app must be restarted by its operator to load new native code.

For frontend-only builds: `trunk build --release`. If the surrounding shell sets
`NO_COLOR=1`, use `NO_COLOR=true trunk build --release` (Trunk expects a boolean).
The browser build supports encrypted identity import, session unlock, explicit
Lock, verified recent-history sync, confirmed spendable balances and exact-fee
publication through direct public Esplora requests. It uses the same URMA vault,
record proof, wallet signing and immutable-plan validation as native Wire. No node
or miner is started by the app.

## Browser offline operation

Historical evidence recorded in README revision
`ef850ea09c6d50d352d35c4902e937c67424cafe` (not a claim of current verification;
the execution date and tested source revision are not recorded here): the browser
engine passed synthetic-chain publication,
restart and offline tests. A direct browser read of Litecoin Testnet block
4892707 passed transparent transaction Merkle and witness commitment verification
with the shared Litecoin decoder. Esplora's transparent block encoding is handled
explicitly, separately from full node serialization. The result is a transparent
read projection; it is not a full MWEB consensus verifier. Confidential MWEB
transactions remain unsupported, and extension contents are not authenticated by
this reader. This read-only check is not evidence of a live browser publication.
These observations concern the locally integrated implementation; the committed
patches select the sibling core checkout rather than establish published-package
equivalence.

Serve the complete `trunk build --release` output at an origin's root over HTTPS
(localhost is accepted for development). Python 3 is required by the Trunk
post-build manifest hook. It includes every emitted file, including hashed WASM,
JavaScript snippets, CSS and images. The service worker verifies every file's
SHA-256 before marking its version complete. Only then does the application show
its offline-ready message. Interrupted or changed downloads cannot
replace the previous complete version. Updates activate when older tabs close;
previous complete caches are retained. Deep-link reload uses the cached shell
even if the hosting server is unavailable. There are no remote startup fonts.

The imported URMA vault bytes persist in IndexedDB, separate from public asset
caches. Import verifies the password and refuses to overwrite an existing vault,
including concurrent imports from different tabs. Keys are held only in WASM
session memory and dropped on Lock/reload; neither the service worker nor browser
storage holds the password or unlocked keys. Each tab has its own locked session.
The storage button requests persistent storage and reports a denial honestly.
Keep the original encrypted vault backup: browser data can still be deleted by
the user, browser policy or storage eviction. No persistent auto-unlock exists.

Settings selects a public Litecoin Testnet Esplora API URL. Sources must support
HTTPS and CORS. See the [configuration sources](#configuration-and-state) for
endpoint defaults.
Wire checks genesis before using a source and never silently replaces a configured
source. Fetch uses no cookies or backend proxy. Unreachable sources, CORS denial,
rejections and malformed proofs produce explicit errors. Browser privacy policy
and endpoint availability can still prevent a connection.

IndexedDB stores the recent-history starting height, contiguous block-hash
checkpoints and verified records. Refresh resumes from the saved checkpoint,
rolls back changed hashes, and saves each verified block before advancing, in
bounded slices per refresh. Transparent block Merkle roots, witness commitments and URMA
author proofs are checked with the shared library. Cached history remains
readable offline with a stale-source notice; it is not marked caught up. This
uses public-provider chain observations, not an independently validating node or
full-chain history. No offline transaction confirmation is claimed.

Funding checks exact raw transaction amounts/scripts, block inclusion, unspent
status and coinbase maturity. Preparing stores an immutable signed plan with an
exact quoted fee, without submitting. Approval saves that plan's validated ID
before any broadcast. The journal persists submission intent and observations;
restart or a lost response reconciles the same signed bytes. Reveal submission
follows as soon as the commit is observed in the mempool or confirmed. Success
still requires both transactions to confirm. The public Esplora path has no
mempool preflight; policy rejection pauses publication without fee changes. Approved plans resume
while the vault is locked. Offline plan views show saved bytes and explicitly
unknown current confirmation status. Origin-wide Web Locks serialize chain
operations across tabs, and an unfinished approved plan blocks new preparation.
Browsers without Web Locks fail safely for these operations.

Focused nonvisual browser fixture check, using installed Chromium/chromedriver
and a disposable profile (no installation, live chain source or user vault):

```sh
NO_COLOR=true trunk build --features fixture-harness --offline
python3 scripts/browser_fixture_test.py --chain
node scripts/offline_worker_test.js
```

The optional fixture feature exports a local test harness; omit it from normal
builds. It checks the committed public vault, password failure, signature
verification, explicit Lock, duplicate import, actual origin-server shutdown and
deep-link reload, retained ciphertext and locked reload. Mocked Esplora checks
cover balances, bounded sync, exact approval, accepted-but-lost broadcast replies,
restart with a locked vault, commit-before-reveal, reorg rollback, corrupt block
rejection, operation locking, and offline cached history/plans. No live broadcast
is used. Optional `--public-probe` only fetches a public genesis, tip and block to
check actual browser CORS and block decoding. Run it separately from `--chain`,
so real-provider observations never mix with synthetic browser state. These
checks take no screenshots.

## Branding

`assets/urma-wire.svg` contains the existing URMA Wire mark with its ROSINT eagle
embedded, so it has no external asset dependency. The header and browser icon
use this animated SVG (with its reduced-motion variant). The static
`assets/urma-wire-icon.png` supplies the Tauri desktop/mobile icons. Regenerate
those sizes with `cargo tauri icon assets/urma-wire-icon.png`.

## Testnet flow

1. Identity → explicitly import an encrypted URMA vault or unlock the configured
   active URMA identity with its vault password. Desktop also offers **Use
   configured unlock file** when present. No identity is created. Wallet shows
   network funding separately from the identity session.
2. Write a post/reply → **Review & publish** prepares and saves signed bytes,
   without broadcast. Optional wire and hashtag inputs are structured protocol
   fields. The review displays the exact frozen text, wire, hashtags, reply target,
   total fee, selected funding and value returned. See the
   [configuration sources](#configuration-and-state) for the fee policy; there is
   no automatic fee increase. Funding currently
   needs one adequate confirmed spendable output; total balance is insufficient
   to prove that such an output exists.
3. **Approve fee & publish** approves that exact plan and begins submission.
   The commit is submitted first; once observed in the mempool or confirmed,
   its reveal is submitted in the same pass. The frontend periodically reconciles
   and reports both transactions; success requires both confirmed.
   A source error or rejection stops following with its reason. Resolve it and
   choose **Resume publication**; no generic infinite retry or fee bump occurs.
4. Completion refreshes the confirmed feed. **Refresh confirmed feed** also
   advances the index explicitly. No mempool record is inserted into the feed.

Pause stops after the in-flight request. Closing Wire stops following; reopen
and resume the saved signed plan without unlocking again. Locking the identity
prevents new signing but does not revoke an already approved signed plan.
The app admits one unfinished approved publication and one instance per data
directory, preventing concurrent app plans from reusing funding.

## Configuration and state

The configuration entry points are [app-core](app-core/src/config.rs),
[app-engine](app-engine/src/config.rs) and [app-ui](app-ui/src/config.rs).
Consult these sources for changing bounds, defaults and timing; the flows below describe
their behavioral purpose.

Reads `URMA_CONFIG`, or `~/.config/urma/config.json`: `rpc_url`, `node_auth_file`,
`vault`, `unlock_file`. RPC/auth environment overrides are `URMA_RPC_URL` and
`URMA_NODE_AUTH_FILE`. Both RPC fields must be supplied together; otherwise the
URMA public Litecoin Testnet source pool is used. Configured RPC failure never
silently redirects credentials to a public endpoint. Genesis is checked.
Settings are loaded at startup; restart after changing configuration.

Default vault/unlock paths match the CLI. Passwords are never saved by Wire.
The Identity page copies a validated encrypted vault into app-data `identity.urma`,
subject to the shared URMA vault format validation. Import requires its password,
validates its active author before writing, and never overwrites an existing
imported vault or the CLI vault. The imported copy takes precedence for future
unlocks. Its keys
stay in the running session until Lock or process exit; reloading the frontend
only reads public session status and never automatically unlocks. A source or
balance failure does not lock the identity or change its author.
State is under Tauri's platform app-data directory for `org.urma.wire`:
`plans/<plan-id>/plan.json`, exact approval, progress journal, `active.json`,
`ltc-testnet-index.json` and `ltc-testnet-start.json`. Existing prototype
`wire_store.json` is left untouched and not used as verified chain evidence.

First sync starts within a bounded recent range before the current tip and remembers
that height. Each refresh processes a bounded slice, with the indexed range and remaining
catch-up shown explicitly. This is a recent-history feed, not full-chain search.
The library verifies record/author proofs and rolls back orphaned index entries.
Posts and replies support an inert Markdown subset (text/code/paragraphs and
external image references); HTML is displayed as text. Bitcoin UI integration
remains separate work.

## Public profiles and pixel-art avatars

Identity includes a nickname editor and a user-authored 32×32 drawing editor,
using exactly the reference 16-color palette. Click/tap paints pixels; mouse
dragging paints continuously. The preview and signed review use the same inert
renderer. Nicknames preserve submitted UTF-8 exactly, including empty names;
names are not unique. Empty/missing names fall back to the key fingerprint.

Nickname (kind 05) and avatar (kind 06) updates each use the existing atomic
quote → exact fee approval → commit/reveal → resume flow. The avatar is 512
row-major pixel bytes, high nibble first, in a 520-byte URMA record; transaction
overhead is additional. Review decodes the saved signed reveal, including its
frozen avatar pixels, rather than reading the current editor. Both native and
browser publishing use the same record construction and review code.

Profiles and avatars are resolved independently from the selected network's
verified confirmed index, ordered by block height and transaction position.
Every feed refresh rebuilds this disposable state, including after reorgs, and
cards/replies/profile pages use it with fingerprint/identicon fallbacks. This
inherits the current bounded history range: older identity updates outside that
range are unavailable. **Load confirmed profile** copies the indexed state into
the editor; drafts remain local and are not saved across page changes.

## External images

Published photo references use the existing Markdown image representation
internally; the **Add photo** control does not require writing that syntax.
Native checked HTTP/HTTPS fetching, including
public IP hosts. DNS answers must all be public unicast; each redirect destination
is resolved and checked separately. Connections are pinned to those addresses,
environment proxies are disabled, and redirects are bounded. HTTP connection and
request timeouts are configured; OS DNS resolution is separate and can outlast
the request timeout. No cookies, credentials or referring post URL are
sent. Fetching still exposes the user's network address and request metadata.

PNG, JPEG, WebP, GIF, BMP, ICO, TIFF and AVIF are enabled. The response must
declare an `image/*` type, but the decoder is chosen from the signature, so a
host that transcodes or mislabels its body cannot select the codec. Download,
dimensions, total pixels, decoder allocation and re-encoded output are bounded;
see the [configuration sources](#configuration-and-state).
Limits also apply while reading image dimensions. Rendering re-encodes inert
pixels and discards source metadata/animation; SVG and arbitrary embeds are
unsupported. Image references, concurrent requests and displayed data URLs plus estimated
RGBA pixels have bounded budgets (not a process-wide memory guarantee).

The **Don't autoload external images** preference persists in WebView
localStorage; disabled images show an explicit **Load image** control. When
autoload is enabled, opening a post can contact its image host and expose the
user's network address.
Preference failures are reported and a failed initial read disables autoload.
The browser build explicitly disables image fetching because browsers cannot
enforce DNS pinning and redirect checks; HTTPS pages additionally explain HTTP
mixed-content refusal. It uses no media proxy. Profile/avatar publication and
pixel rendering still work in both runtimes.

Wire retains no HTTP, disk or application media cache. Only currently displayed
images live in memory. **Refresh image**
discards its display and requests the current resource again; **Clear displayed
media** removes displayed data and invalidates in-flight results. Hosts can change
or remove images; Wire does not claim historical preservation.

## Wires, hashtags and local follows

A wire such as `/rosint` is an open channel, with no owner or registration;
using a label never proves affiliation. Each post/reply can name one wire and
up to eight hashtags such as `#investigation`, shared across wires on the same
network. Leaving both fields empty keeps legacy kind 02/04 encoding. Structured
records use new kinds 0A/0B. Names use 1–32 lowercase ASCII letters/digits, `-`
or `_`, beginning with a letter/digit; the UI lowercases label input and sorts
and deduplicates hashtags before exact approval. Text remains unchanged.
Cards show a carried label where the text writes it, and that word follows or
unfollows it; only labels carried without appearing in the text are shown as
separate chips. This is display only: the signed text and topics are unchanged.

Global feed includes all records and clears both optional filters. Separate wire
and hashtag controls work independently: either alone filters that dimension;
both together require a record to match both (AND). Selecting or clearing one
does not change the other. Each Follow button subscribes only to its individual
wire or hashtag, never a combined filter. Following remains a separate view of
the union of locally followed labels (OR). Filters apply before taking the
bounded latest results. Follows persist in WebView localStorage, keyed by chain and
network name, with a bounded local capacity; no transaction or payment is involved.
They are tied to this local WebView profile. Storage failures are shown.
The initial network options are fixed and only Litecoin Testnet is enabled.

Index format 2 rescans a format-1 cache from its recorded starting height, because
old readers could have skipped the newly allocated kinds. This occurs on the
next explicit sync, uses the normal bounded slices and retains chain validation.
No full-chain history or reduced blockchain scanning cost is implied.

## Replies in the feed

A reply whose parent is shown in the feed sits under that parent instead of
the top-level stream. The newest replies appear first, up to Settings →
**Replies shown per post** (0 keeps them collapsed, persisted in
WebView localStorage); **View all N replies** opens the whole thread. A reply
to a reply stays inside its parent reply's thread. A reply whose parent is
outside the indexed history, or filtered out, stays top-level and names the
record it answers. Grouping is display only: records and the index are unchanged.

## Focused local checks

The commands below are reproduction instructions, not evidence that checks were
rerun for this documentation update. Test coverage descriptions are recorded in
README revision `ef850ea09c6d50d352d35c4902e937c67424cafe`; they do not establish
the execution date or tested source revision.

Coin tab logos are bundled locally: Litecoin from
https://github.com/spothq/cryptocurrency-icons/blob/master/svg/color/ltc.svg
(CC0), and Bitcoin from
https://github.com/bitpay/bitcoin-brand/blob/master/bitcoin.svg.

The [UI manifest](app-ui/Cargo.toml) records the Thaw and Leptos dependencies.
The network selector uses Thaw Menu;
the four choices remain visible while only Litecoin Testnet is enabled.

The sidebar logo plays on load and periodically thereafter. Future UI
events can replay it with `window.dispatchEvent(new Event("urma:logo-animate"))`.
Each replay uses the trace animation and rests on the green mark. Reduced-motion
preferences disable the animation. The timer and
event listener are removed when the navigation component unmounts.

```sh
cargo test --workspace --locked
cargo check --workspace --locked
cargo check -p wire-app-ui --target wasm32-unknown-unknown --locked
lawkeeper
```

Crates: `app-core` holds the domain types and the typed `Request`/`Response`/`Fault`
contract; `app-engine` holds the single projection and record codec plus the
native (Tauri) and browser (IndexedDB/Esplora) backends; `app-ui` is Leptos only
and asks the engine through `wire_app_engine::client::ask`; `src-tauri` exposes one
`engine` command. `law.toml` declares the layer map and one `config.rs` sanctum
per crate; `scripts/law_report.sh` tallies violations by file and rule.

Tests use temporary files, public fixture identities and a loopback RPC mock;
they do not start a blockchain node or broadcast to a live network. They cover
exact approval, post/reply preparation, confirmation stages, read-only status,
restart, unchanged retransmission, rejection/source failure and feed projection.
Focused profile/media checks are `cargo test -p wire-app-engine --test live
--test public_identity --test media --locked`: exact profile/avatar approval and
resume, fixed bytes/palette, colliding names/latest ordering/rollback projection,
destination/redirect bounds, MIME/signature/decode/size failures and inert browser
capability errors. These are local fixtures, not live media-host acceptance or
visual UI validation.
They do not establish live acceptance of this desktop application.
