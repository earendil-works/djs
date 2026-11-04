//! `using` declarations around a continuation's site (D2876). The resources a scope
//! holds are added again at the start of K's scope, in their order, through
//! `$djs_U(value, method, kind)` (an object whose dispose method calls the saved method
//! on the saved value), so the language disposes them as the original would. A site in
//! the disposal itself is a scope of K that adds the resources not yet disposed, then the
//! one being awaited as the last (so disposed first: its awaited value, or a `null` one
//! for the await a sync disposal after a `null` resource takes), and completes as the
//! original's scope did (its exception thrown, its return or jump taken).

use alloc::string::String;
use alloc::vec::Vec;

use super::*;

impl G<'_, '_> {
    /// The `using` record of the scope path node `i` opens: the one named by an offset
    /// in the node and in no node nested in it on the path.
    fn using_record(&self, i: usize) -> Option<usize> {
        let n = self.node(i);
        let (a, b) = (n.start, n.end);
        let deeper: Vec<(u32, u32)> = (i + 1..self.act.site.path.len())
            .map(|j| (self.node(j).start, self.node(j).end))
            .collect();
        self.act.usings.iter().position(|u| {
            let Some(p) = u
                .slot
                .rsplit_once('@')
                .and_then(|(_, p)| p.parse::<u32>().ok())
            else {
                return false;
            };
            a <= p && p < b && !deeper.iter().any(|&(x, y)| x <= p && p < y)
        })
    }

    /// The declarations that add resources `triples` again. K's scope record stands
    /// for the original's at offset `fpos` (a `using` record, named by its first
    /// binding).
    fn using_decls(&mut self, triples: &[(String, String, i32)], fpos: u32) -> R<P> {
        let mut parts = Vec::new();
        for (x, (v, m, k)) in triples.iter().enumerate() {
            let name = self.new_temp();
            let (kw, init) = match k {
                0 => ("using ", cat!("$djs_U(", v, ", ", m, ", 0)")),
                2 | 4 => ("await using ", cat!("$djs_U(", v, ", ", m, ", ", *k, ")")),
                3 => ("await using ", String::from("null")),
                _ => return no("a using resource of a stack's kind"),
            };
            parts.push_out(s(kw));
            if x == 0 {
                parts.push_out(self.mark("using", fpos));
            }
            parts.push_out(P::S(cat!(name, " = ", init, ";\n")));
        }
        Ok(P::L(parts))
    }

    /// The resources of the scope path node `i` opens, added again at its start in K
    /// (the site in the scope's code, no disposal begun).
    pub(super) fn using_readd(&mut self, i: usize) -> R<P> {
        let Some(u) = self.using_record(i) else {
            return Ok(s(""));
        };
        let u = self.act.usings[u].clone();
        // (Its value is the resources this writes: nothing to read.)
        if !self.used.contains(&u.slot) {
            self.used.push_out(u.slot.clone());
        }
        if !u.state.is_empty() {
            return no("a using scope's disposal state at a site in its code");
        }
        self.using_decls(&u.triples, slot_pos(&u.slot))
    }

    /// The rest of the disposal of the scope path node `i` opens (the site is its
    /// disposal's await), and the completion that entered it.
    pub(super) fn using_disposal(&mut self, i: usize) -> R<P> {
        let u = self
            .using_record(i)
            .ok_or("plain dumps: a disposal without its using record")?;
        let u = self.act.usings[u].clone();
        // (Its value is the resources this writes: nothing to read.)
        if !self.used.contains(&u.slot) {
            self.used.push_out(u.slot.clone());
        }
        let field = |k: &str| u.state.iter().find(|x| x.0 == k).map(|x| x.1.clone());
        let next = field("next").and_then(|v| v.parse::<i64>().ok());
        let Some(next) = next else {
            return no("a using disposal without its place");
        };
        // The resources not yet disposed (the one at `next` on).
        let left = usize::try_from(next / 3 + 1)
            .unwrap_or(0)
            .min(u.triples.len());
        // The one awaited now, disposed first: its value awaited again, a `null` one's
        // await, or a sync disposal's (kind 2) exception thrown after the await.
        let awaited = field("hasAwaited").is_some_and(|v| v == "true");
        let mut triples = u.triples[..left].to_vec();
        if field("hasPending").is_some_and(|v| v == "true") {
            let e = field("pending").unwrap_or_else(|| String::from("undefined"));
            let c = self.new_temp();
            self.temps.push_out(cat!(c, " = ", e));
            triples.push_out((
                String::from("undefined"),
                cat!("() => { throw ", c, "; }"),
                2,
            ));
        } else if awaited {
            let v = field("value").unwrap_or_else(|| String::from("undefined"));
            // (A constant: K's text runs once the dump's names are gone.)
            let c = self.new_temp();
            self.temps.push_out(cat!(c, " = ", v));
            triples.push_out((String::from("undefined"), cat!("() => ", c), 4));
        } else {
            triples.push_out((String::new(), String::new(), 3));
        }
        let decls = self.using_decls(&triples, slot_pos(&u.slot))?;
        if field("needsAwait").is_some_and(|v| v == "true") && !awaited {
            return no("a using disposal waiting for a null resource's await (0203)");
        }
        // The completion: the exception the disposal carries, else the scope's own.
        let pos = slot_pos(&u.slot);
        let exit = if field("hasError").is_some_and(|v| v == "true") {
            let e = field("error").unwrap_or_else(|| String::from("undefined"));
            let c = self.new_temp();
            self.temps.push_out(cat!(c, " = ", e));
            for x in ["completion.kind", "completion.value"] {
                let n = cat!(x, "@", pos);
                if self.has(&n) {
                    self.take(&n)?;
                }
            }
            cat!("throw ", c, ";")
        } else {
            let kname = cat!("completion.kind@", pos);
            let kind = if self.has(&kname) {
                self.take(&kname)?
            } else {
                String::from("0")
            };
            let vname = cat!("completion.value@", pos);
            if kind == "0" && self.has(&vname) {
                self.take(&vname)?;
            }
            match kind.parse::<i32>() {
                Ok(0) => String::new(),
                Ok(2) => match self.tmp("completion.value", pos)? {
                    P::S(v) => cat!("return ", v, ";"),
                    _ => return no("a using scope's return value"),
                },
                Ok(c) if c >= 3 => {
                    let n = cat!("completion.value@", pos);
                    if self.has(&n) {
                        self.take(&n)?;
                    }
                    self.jump_to(i + 1, ((c - 3) / 2) as u32, c % 2 == 0)?
                }
                _ => return Err(cat!("plain dumps: a using scope's completion ", kind)),
            }
        };
        let m = self.mark("site", self.act.site.pos);
        // (In a block of its own: its constants come before it, and it stands where a
        // statement does, a `try`'s block included.)
        let st = self.with_go(P::L(alloc::vec![s("{\n"), decls, P::S(exit), s("\n}"), m]));
        Ok(P::L(alloc::vec![s("{\n"), st, s("\n}")]))
    }
}

/// The offset a slot is named by (`using.record@39`).
fn slot_pos(slot: &str) -> u32 {
    slot.rsplit_once('@')
        .and_then(|x| x.1.parse().ok())
        .unwrap_or(0)
}
