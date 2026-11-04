//! The continuation of a suspended activation (D2830, docs/plain-js-dumps.md §2): a
//! function of the original's kind whose body is the rest of the original from its
//! site, written from the original's text by the syntax the compiler recorded around
//! the site (D2850, compiler/smap_syntax.rs).

use alloc::string::String;

use super::plain::{Act, Ctx, KText};

/// Writes activation `act`'s continuation.
pub(super) fn generate(act: &Act, ctx: &mut dyn Ctx) -> Result<KText, String> {
    let _ = ctx;
    if act.site.path.is_empty() {
        return Err(String::from("plain dumps: no syntax recorded at the site"));
    }
    Err(String::from(
        "plain dumps: continuations are not written yet",
    ))
}
