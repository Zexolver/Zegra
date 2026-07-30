//! GameJolt integration status.
//!
//! Unlike Steam/GOG/Epic/itch.io, GameJolt does not expose any API suitable
//! for "list the games this user owns or has installed":
//!
//! - GameJolt's official "Game API" (<https://gamejolt.com/game-api/doc>) is
//!   scoped *per game* behind that game's own private key, and is meant for a
//!   game binary to report trophies/scores/data-store entries back to
//!   GameJolt — it has no concept of a user-wide library.
//! - GameJolt discontinued its standalone desktop client, so there is no
//!   local install-manifest format to read either (unlike Steam/GOG/Epic).
//! - There is no public, documented, generally-available "my library" or
//!   catalog endpoint we could call with just a user's own credentials.
//!
//! Rather than scraping undocumented internal endpoints that could break or
//! change without notice (and which we could not verify are meant for
//! third-party use), Zegra reports this platform as unsupported so the UI can
//! say so honestly instead of silently showing an empty/broken library.

pub const UNSUPPORTED_REASON: &str =
    "GameJolt has no public API for listing a user's owned or installed games \
     (its Game API is scoped per-game via a private key, for trophies/scores only). \
     GameJolt support is not available yet.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_is_non_empty() {
        assert!(!UNSUPPORTED_REASON.is_empty());
    }
}
