//! `yield*` at a continuation's site (D2875): the delegation goes on as a `yield*` of K
//! over a wrapper of the saved inner record, `$djs_Y(it, next, async, ...first)`. Its
//! first step is the inner result the original waits for (an async generator awaiting
//! the inner `next()`'s or `throw()`'s promise), or a placeholder where the original is
//! suspended at the delegation's yield (K is primed to it); later steps, `throw` and
//! `return` are the inner iterator's own, looked up when the delegation uses them.

use super::*;
use crate::compiler::SiteKind;

impl G<'_, '_> {
    pub(super) fn yield_star_site(&mut self, n: &Node) -> R<P> {
        self.yield_star_await_ok()?;
        let site = &self.act.site;
        let first = match site.kind {
            SiteKind::YieldStar => None,
            // Awaiting the inner result of a `next` or `throw` (the last two of the
            // inner-result awaits): the delegation's loop goes on from it.
            SiteKind::YieldStarAwait if site.pos == n.end => Some(self.ctx.awaiting()),
            _ => return Err(cat!("plain dumps: a site of kind ", site.name, " (0203)")),
        };
        let asynch = self.act.kind == suspend::ASYNC_GENERATOR;
        let it = self.tmp("yield*.iterator", n.start)?;
        let next = self.tmp("yield*.next", n.start)?;
        // K's `yield*` maps to the original's at its start (its yield) and its end (the
        // awaits of inner results).
        let at_start = self.mark("site", n.start);
        let at_end = self.mark("site", n.end);
        let mut out = alloc::vec![s("("), at_start, P::V(n.start, n.start + 6), s(" $djs_Y(")];
        out.push_out(it);
        out.push_out(s(", "));
        out.push_out(next);
        out.push_out(s(if asynch { ", true" } else { ", false" }));
        if let Some(p) = first {
            out.push_out(P::S(cat!(", ", p)));
        }
        out.push_out(s(")"));
        out.push_out(at_end);
        out.push_out(s(")"));
        Ok(P::L(out))
    }

    /// Which await of an inner result an async `yield*` site is: the inner results come
    /// after `next`, `return` and `throw`, in that order, at the node's end.
    fn yield_star_await_ok(&self) -> R<()> {
        let site = &self.act.site;
        if site.kind != SiteKind::YieldStarAwait {
            return Ok(());
        }
        let nth = self
            .act
            .peers
            .iter()
            .position(|x| *x == site.name)
            .ok_or("plain dumps: a yield* await without its peers")?;
        // 0: after `next`, 1: after `return` (its result decides between a return and
        // a yield, refused), 2: after `throw`.
        if nth == 1 {
            return no("a yield* waiting for its inner return() (0203)");
        }
        Ok(())
    }
}
