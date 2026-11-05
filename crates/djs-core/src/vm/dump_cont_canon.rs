//! Continuations whose layout a re-dump makes again (D2913).
//!
//! Origin tracking reads a continuation `K`'s frame as one of its origin `f` through K's
//! layout: where K copies `f`'s text (`segs`), where its own constructs are (`scaf`) and
//! what stands for `f`'s temporaries (`cmap`). K's record does not hold that layout; it
//! holds what makes it again from `f`: the site (`at`), the decisions the writer took on
//! values that shape K (`r`, the tape: which operands K reads again, a completion's kind,
//! …) and where K's text has generated text that depends on values (`h`, the holes).
//!
//! K is written as pieces: generated text (runs), copies of `f`'s text, marks of K's own
//! constructs and literals standing for temporaries, a run between any two others. The
//! writer writes K, then writes it again from a canonical activation: the site and its
//! slot names, every value a placeholder, with the tape. The two must have the same
//! pieces but for the texts of runs and literals; those that differ are the holes,
//! `[piece, length, ...]`. A re-dump writes the canonical K again, checks its fixed text
//! against K's own text, takes the holes' texts from it and so has the writer's pieces,
//! whose layout is the writer's.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use super::super::plain::{Act, BDecl, Ctx, KText, Slot, Using};
use super::{G, P, R, join, run};
use crate::buf::PushOut;
use crate::compiler::{Site, Special, Syntax};

/// A value of the canonical activation.
pub(in super::super) const PH: &str = "\u{1}";

/// A piece of a continuation's text.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum Piece {
    Run(String),
    /// The original's text `[a, b)`.
    Copy(u32, u32),
    /// Where scaffold record `scaf[i]` is.
    Mark(usize),
    /// A temporary's literal, `cmap[i]`.
    Lit(usize, String),
}

/// A continuation as pieces, with its records.
pub(super) struct Out {
    pub pieces: Vec<Piece>,
    /// Kind, offset in the original, offset in K (set by [`layout`]), the rest of the
    /// record (JSON).
    pub scaf: Vec<(&'static str, u32, u32, String)>,
    /// A temporary of the original: slot, K's binding or the literal, whether literal.
    pub cmap: Vec<(String, String, bool)>,
    pub used: Vec<String>,
    pub args: Vec<String>,
    pub tape: Vec<u32>,
    /// A method's home object's prototype: K's text goes in `({ __proto__: p, K }).K`.
    pub wrap: Option<String>,
}

impl G<'_, '_> {
    /// The continuation's pieces: `parts` rendered (no blank lines of K's own), K's
    /// helpers declared where the marker is.
    pub(super) fn flatten(self, parts: P) -> Out {
        let mut r = Render {
            ftext: &self.act.ftext,
            pieces: Vec::new(),
            run: String::new(),
            nl: false,
            helpers: None,
        };
        r.walk(&parts);
        let tail = core::mem::take(&mut r.run);
        r.pieces.push_out(Piece::Run(tail));
        let mut text = String::new();
        for p in &r.pieces {
            match p {
                Piece::Run(t) | Piece::Lit(_, t) => text.push_str(t),
                Piece::Copy(a, b) => {
                    text.push_str(r.ftext.get(*a as usize..*b as usize).unwrap_or(""))
                }
                Piece::Mark(_) => {}
            }
        }
        // K's helpers, captured where the marker is.
        let act = self.act;
        let names: Vec<&str> = [
            "R", "G", "C", "D", "A", "L", "N", "Q", "X", "O", "I", "E", "P", "S", "Y", "U", "DD",
        ]
        .into_iter()
        .filter(|h| text.contains(&cat!("$djs_", *h, "(")))
        // A continuation of a continuation declares them already (its own
        // first line, restored from its slot).
        .filter(|h| !act.slots.iter().any(|s| s.name == cat!("$djs_", *h)))
        .collect();
        let mut decl = String::new();
        if !names.is_empty() {
            let list: Vec<String> = names
                .iter()
                .map(|h| cat!("$djs_", *h, " = $djs.", *h))
                .collect();
            decl = cat!("const ", join(&list, ", "), ";\n");
        }
        if self.uses_arguments && !text.contains("var $djs_arguments = ") {
            decl.push_str("const $djs_arguments = undefined;\n");
        }
        let mut pieces = r.pieces;
        if let Some((i, at)) = r.helpers
            && let Some(Piece::Run(t)) = pieces.get_mut(i)
        {
            t.insert_str(at, &decl);
        }
        Out {
            pieces,
            scaf: self.scaf,
            cmap: self.cmap,
            used: self.used,
            args: Vec::new(),
            tape: self.tape,
            wrap: None,
        }
    }
}

struct Render<'t> {
    ftext: &'t str,
    pieces: Vec<Piece>,
    run: String,
    /// The text so far ends a line.
    nl: bool,
    /// Where the helpers go: the piece (the run being written) and the offset in it.
    helpers: Option<(usize, usize)>,
}

impl Render<'_> {
    fn text(&mut self, t: &str) {
        let t = if self.nl {
            t.trim_start_matches('\n')
        } else {
            t
        };
        if !t.is_empty() {
            self.run.push_str(t);
            self.nl = t.ends_with('\n');
        }
    }

    fn item(&mut self, p: Piece, text: &str) {
        let run = core::mem::take(&mut self.run);
        self.pieces.push_out(Piece::Run(run));
        self.pieces.push_out(p);
        if !text.is_empty() {
            self.nl = text.ends_with('\n');
        }
    }

    fn walk(&mut self, p: &P) {
        match p {
            P::S(t) | P::Decl(_, t) => self.text(t),
            P::Scope(names, braced, q) => {
                let braces = *braced && !names.is_empty();
                if braces {
                    self.text("{\n");
                }
                self.walk(q);
                if braces {
                    self.text("\n}\n");
                }
            }
            P::V(a, b) => {
                if b > a {
                    let t = self.ftext.get(*a as usize..*b as usize).unwrap_or("");
                    self.item(Piece::Copy(*a, *b), t);
                }
            }
            P::Lit(i, t) => self.item(Piece::Lit(*i, t.clone()), t),
            P::Mark(i) => self.item(Piece::Mark(*i), ""),
            P::Helpers => self.helpers = Some((self.pieces.len(), self.run.len())),
            P::L(list) => {
                for q in list {
                    self.walk(q);
                }
            }
        }
    }
}

/// The text of `pieces` and its segments `[kStart, fStart, len]`; sets the scaffold
/// records' offsets in K and the literals' texts in `cmap`.
pub(super) fn layout(
    pieces: &[Piece],
    ftext: &str,
    scaf: &mut [(&'static str, u32, u32, String)],
    cmap: &mut [(String, String, bool)],
) -> (String, Vec<u32>) {
    let mut text = String::new();
    let mut segs: Vec<u32> = Vec::new();
    for p in pieces {
        match p {
            Piece::Run(t) | Piece::Lit(_, t) => {
                if let Piece::Lit(i, _) = p
                    && let Some(c) = cmap.get_mut(*i)
                {
                    c.1 = t.clone();
                }
                // Text that goes on as the original's right after a copied stretch is
                // part of it (one segment, not two).
                let at = text.len() as u32;
                if let [.., ks, fs, len] = segs[..]
                    && ks + len == at
                {
                    let f = ftext.get((fs + len) as usize..).unwrap_or("").as_bytes();
                    let same = t.bytes().zip(f).take_while(|(x, y)| x == *y).count();
                    // (Whole characters.)
                    let same = (0..=same)
                        .rev()
                        .find(|&n| t.is_char_boundary(n))
                        .unwrap_or(0);
                    let n = segs.len();
                    segs[n - 1] += same as u32;
                }
                text.push_str(t);
            }
            Piece::Copy(a, b) => {
                let at = text.len() as u32;
                match segs[..] {
                    // (A copy that goes on the last one.)
                    [.., ks, fs, len] if ks + len == at && fs + len == *a => {
                        let n = segs.len();
                        segs[n - 1] += b - a;
                    }
                    _ => {
                        segs.push_out(at);
                        segs.push_out(*a);
                        segs.push_out(b - a);
                    }
                }
                text.push_str(ftext.get(*a as usize..*b as usize).unwrap_or(""));
            }
            Piece::Mark(i) => {
                if let Some(r) = scaf.get_mut(*i) {
                    r.2 = text.len() as u32;
                }
            }
        }
    }
    (text, segs)
}

/// Writes activation `act`'s continuation and checks that its canonical activation makes
/// it again (D2913).
pub(super) fn generate(act: &Act, ctx: &mut dyn Ctx) -> R<KText> {
    let mut w = run(act, ctx, None)?;
    let c = canonical(
        &act.site,
        act.syntax.clone(),
        act.ftext.clone(),
        act.fkind,
        act.kind,
        act.peers.clone(),
    );
    let again = |e: String| {
        cat!(
            "plain dumps: a continuation its origin does not make again (",
            e,
            ")"
        )
    };
    let r = run(&c, &mut Canon { act: &c }, Some(&w.tape)).map_err(again)?;
    let h = holes(&w, &r).map_err(again)?;
    let (text, _) = layout(&w.pieces, &act.ftext, &mut w.scaf, &mut w.cmap);
    let text = match &w.wrap {
        Some(p) => cat!("({ __proto__: ", p, ", ", text, " }).K"),
        None => text,
    };
    Ok(KText {
        text,
        used: w.used,
        args: w.args,
        holes: h,
        tape: w.tape,
    })
}

/// Where `w`'s pieces differ from `r`'s, which has the same shape: `[piece, length]` of
/// each run or literal whose text differs.
fn holes(w: &Out, r: &Out) -> R<Vec<u32>> {
    if w.pieces.len() != r.pieces.len() {
        return Err(cat!(w.pieces.len(), " pieces, ", r.pieces.len(), " again"));
    }
    let mut h = Vec::new();
    let mut lits = Vec::new();
    for (i, (a, b)) in w.pieces.iter().zip(&r.pieces).enumerate() {
        let differs = |x: &String, y: &String, h: &mut Vec<u32>| {
            if x != y {
                h.push_out(i as u32);
                h.push_out(x.len() as u32);
            }
        };
        match (a, b) {
            (Piece::Run(x), Piece::Run(y)) => differs(x, y, &mut h),
            (Piece::Lit(i, x), Piece::Lit(j, y)) if i == j => {
                lits.push_out(*i);
                differs(x, y, &mut h)
            }
            (Piece::Copy(..), Piece::Copy(..)) | (Piece::Mark(_), Piece::Mark(_)) if a == b => {}
            _ => return Err(cat!("piece ", i, " differs")),
        }
    }
    let same_scaf = w.scaf.len() == r.scaf.len()
        && w.scaf
            .iter()
            .zip(&r.scaf)
            .all(|(x, y)| x.0 == y.0 && x.1 == y.1 && x.3 == y.3);
    if !same_scaf {
        return Err(String::from("its records differ"));
    }
    let same_cmap = w.cmap.len() == r.cmap.len()
        && w.cmap
            .iter()
            .zip(&r.cmap)
            .enumerate()
            .all(|(i, (x, y))| x.0 == y.0 && x.2 == y.2 && (lits.contains(&i) || x.1 == y.1));
    if !same_cmap {
        return Err(String::from("its temporaries differ"));
    }
    Ok(h)
}

/// The layout of a continuation `ktext` of the original `act` stands for (canonical),
/// from its record's tape and holes: the writer's segments, scaffold records (kind,
/// offset in the original, offset in K, the rest as JSON) and temporaries.
pub(in super::super) struct Layout {
    pub segs: Vec<u32>,
    pub scaf: Vec<(&'static str, u32, u32, String)>,
    pub cmap: Vec<(String, String, bool)>,
}

///
/// `ts`: the original is TypeScript, which the dump wrote as JavaScript, its types blanked
/// (D2879): K's text has spaces where the original's has types.
pub(in super::super) fn layout_of(
    act: &Act,
    tape: &[u32],
    h: &[u32],
    ktext: &str,
    ts: bool,
) -> R<Layout> {
    let mut r = run(act, &mut Canon { act }, Some(tape))?;
    let starts = |at: usize, t: &str| {
        let k = ktext.as_bytes().get(at..at + t.len());
        k.is_some_and(|k| {
            k.iter()
                .zip(t.as_bytes())
                .all(|(&x, &y)| x == y || (ts && x == b' ' && y != b'\n'))
        })
    };
    let mut at = 0usize;
    let mut holes = h.chunks(2);
    let mut next = holes.next();
    let differs = || String::from("its text is not the one its origin makes");
    for (i, p) in r.pieces.iter_mut().enumerate() {
        let hole = match next {
            Some(&[j, len]) if j as usize == i => {
                next = holes.next();
                Some(len as usize)
            }
            _ => None,
        };
        match p {
            Piece::Run(t) | Piece::Lit(_, t) => match hole {
                Some(len) => {
                    let s = ktext.get(at..at + len).ok_or_else(differs)?;
                    *t = String::from(s);
                    at += len;
                }
                None => {
                    if !starts(at, t) {
                        return Err(differs());
                    }
                    at += t.len();
                }
            },
            Piece::Copy(a, b) => {
                let t = act
                    .ftext
                    .get(*a as usize..*b as usize)
                    .ok_or_else(differs)?;
                if hole.is_some() || !starts(at, t) {
                    return Err(differs());
                }
                at += t.len();
            }
            Piece::Mark(_) => {
                if hole.is_some() {
                    return Err(differs());
                }
            }
        }
    }
    if at != ktext.len() || next.is_some() {
        return Err(differs());
    }
    let (_, segs) = layout(&r.pieces, &act.ftext, &mut r.scaf, &mut r.cmap);
    Ok(Layout {
        segs,
        scaf: r.scaf,
        cmap: r.cmap,
    })
}

/// The activation a continuation of `ftext` at `site` is written again from: the
/// site's slots, every value a placeholder.
pub(in super::super) fn canonical(
    site: &Site,
    syntax: Rc<Syntax>,
    ftext: String,
    fkind: u32,
    kind: i32,
    peers: Vec<String>,
) -> Act {
    let slots: Vec<Slot> = site
        .named_slots()
        .into_iter()
        .filter(|s| !super::super::flight::computed_hidden(&s.name))
        .map(|s| Slot {
            name: s.name,
            value: String::from(PH),
            cell: None,
        })
        .collect();
    let usings = slots
        .iter()
        .filter(|s| s.name.starts_with("using.record@"))
        .map(|s| Using {
            slot: s.name.clone(),
            triples: Vec::new(),
            state: Vec::new(),
        })
        .collect();
    let sup = syntax.special.iter().any(|&(k, _)| k == Special::Super);
    Act {
        name: String::new(),
        fexpr: String::new(),
        ftext,
        fkind,
        site: site.clone(),
        syntax,
        awaiting: None,
        this: String::from(PH),
        slots,
        queue: Vec::new(),
        kind,
        proto: None,
        peers,
        usings,
        rereads: Vec::new(),
        super_proto: sup.then(|| String::from(PH)),
    }
}

/// The dump as the canonical activation sees it: every value a placeholder, nothing made
/// later, no code of the dump's own.
struct Canon<'a> {
    act: &'a Act,
}

impl Ctx for Canon<'_> {
    fn slot(&mut self, name: &str) -> Option<String> {
        self.act
            .slots
            .iter()
            .any(|s| s.name == name)
            .then(|| String::from(PH))
    }
    fn cell(&mut self, _: &str) -> Option<Option<String>> {
        None
    }
    fn after_go(&self, _: &str) -> bool {
        false
    }
    fn rereads(&self, _: &str) -> bool {
        false
    }
    fn arg(&mut self, name: &str) -> Option<String> {
        self.slot(name)
    }
    fn made_in(&self, _: &str) -> Option<usize> {
        None
    }
    fn block(&mut self, _: usize, _: &[BDecl]) -> Result<String, String> {
        Ok(String::new())
    }
    fn go(&self, _: usize) -> String {
        String::new()
    }
    fn awaiting(&self) -> String {
        String::from(PH)
    }
}
