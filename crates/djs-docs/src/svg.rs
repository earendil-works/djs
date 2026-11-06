//! Diagrams drawn from the engine's own layout data. Plain SVG strings with an embedded
//! stylesheet (light and dark), so the same file works inline in the site and as an
//! `<img>` on GitHub.

use std::fmt::Write;

use djs_core::heap::layout::CellLayout;

/// The diagrams' stylesheet, shared by the generated diagrams and the hand-drawn ones in
/// docs/guide/img (regen rewrites their `<style>` and `<defs>`). Colors are Elanor's
/// semantic tokens (D2200): inline on the site they follow the page's tokens and its
/// light/dark toggle; as an `<img>` (GitHub), where no token is defined, the fallback is
/// the token's resolved value from assets/elanor/colors.tsv. `$[name]` stands for
/// `--semantic-color-name`.
const RULES: &[(&str, &str)] = &[
    (
        "text",
        "font-family: var(--semantic-font-family-ui, \"Mallos Serif\", Georgia, serif); font-size: 12px; fill: $[text-primary];",
    ),
    (
        ".mono",
        "font-family: var(--semantic-font-family-code, \"Commit Mono\", ui-monospace, Menlo, monospace); font-size: 11px;",
    ),
    (".small", "font-size: 10.5px; fill: $[text-secondary];"),
    (".label", "font-weight: 600;"),
    (".title", "font-weight: 600; font-size: 13px;"),
    (
        ".box, .host",
        "fill: $[background-surface]; stroke: $[border-primary];",
    ),
    (
        ".hdr, .engine",
        "fill: $[background-fill-secondary-information]; stroke: $[border-information];",
    ),
    (
        ".traced, .arena",
        "fill: $[background-fill-secondary-warning]; stroke: $[border-warning];",
    ),
    (
        ".tag",
        "fill: $[background-fill-secondary-positive]; stroke: $[border-positive];",
    ),
    (
        ".cell",
        "fill: $[background-canvas]; stroke: $[border-warning];",
    ),
    (
        ".pad",
        "fill: url(#$NAME-hatch); stroke: $[border-secondary];",
    ),
    (
        ".tail",
        "fill: $[background-surface]; stroke: $[border-primary]; stroke-dasharray: 4 3;",
    ),
    (
        ".zero",
        "fill: $[background-canvas]; stroke: $[border-primary]; stroke-dasharray: 4 3;",
    ),
    (
        ".dead",
        "fill: $[background-surface]; stroke: $[border-secondary]; stroke-dasharray: 3 2;",
    ),
    (".opt", "stroke-dasharray: 3 2;"),
    (".rule", "stroke: $[text-secondary]; stroke-width: 1.2;"),
    (
        ".dash",
        "stroke: $[text-secondary]; stroke-width: 1.2; stroke-dasharray: 4 3;",
    ),
    (
        ".arrow",
        "stroke: $[text-secondary]; fill: none; stroke-width: 1.3; marker-end: url(#$NAME-a);",
    ),
    (
        ".faint",
        "stroke: $[border-primary]; fill: none; stroke-width: 1.1; stroke-dasharray: 4 3; marker-end: url(#$NAME-a);",
    ),
    (".head", "fill: $[text-secondary];"),
    (".hatch", "stroke: $[border-secondary];"),
];

/// `--semantic-color-*` -> (light, dark), resolved by scripts/elanor-update.sh.
const COLORS: &str = include_str!("../assets/elanor/colors.tsv");

fn color(name: &str, dark: bool) -> String {
    let var = format!("--semantic-color-{name}");
    let hex = COLORS
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?, f.next()?, f.next()?))
        })
        .find(|(v, ..)| *v == var)
        .map(|(_, light, d)| if dark { d } else { light })
        .unwrap_or_else(|| panic!("{var} is not in assets/elanor/colors.tsv"));
    format!("var({var}, {hex})")
}

/// `$[name]` -> the token with its light or dark fallback; `$NAME` -> the diagram's name.
fn fill(decls: &str, name: &str, dark: bool) -> String {
    let mut out = String::new();
    let mut rest = decls;
    while let Some(i) = rest.find("$[") {
        out.push_str(&rest[..i]);
        let end = rest[i..].find(']').expect("$[name]") + i;
        out.push_str(&color(&rest[i + 2..end], dark));
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out.replace("$NAME", name)
}

/// The `<style>` and `<defs>` of the diagram `name`. Every selector is scoped to the
/// diagram's root (`svg.dg-NAME`): inline in a page a `<style>` applies to the whole
/// document, and the hand-drawn diagrams use the same class names differently than the
/// page does.
pub fn style(name: &str) -> String {
    style_with(name, &[])
}

/// [`style`] with `extra` rules after the shared ones (the charts' series colors).
pub fn style_with(name: &str, extra: &[(&str, &str)]) -> String {
    let rules = || RULES.iter().chain(extra);
    let scope = |sel: &str| {
        sel.split(", ")
            .map(|s| format!(".dg-{name} {s}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut s = String::from("<style>\n");
    for (sel, decls) in rules() {
        let _ = writeln!(s, "  {} {{ {} }}", scope(sel), fill(decls, name, false));
    }
    s.push_str("  @media (prefers-color-scheme: dark) {\n");
    for (sel, decls) in rules() {
        // Only the colors change.
        let colors: Vec<&str> = decls
            .split(';')
            .map(str::trim)
            .filter(|d| d.contains("$["))
            .collect();
        if !colors.is_empty() {
            let _ = writeln!(
                s,
                "    {} {{ {}; }}",
                scope(sel),
                fill(&colors.join("; "), name, true)
            );
        }
    }
    s.push_str("  }\n</style>\n");
    let _ = writeln!(
        s,
        "<defs><pattern id=\"{name}-hatch\" width=\"6\" height=\"6\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(45)\"><line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"6\" class=\"hatch\"/></pattern>\
         <marker id=\"{name}-a\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path class=\"head\" d=\"M0 0 L10 5 L0 10 z\"/></marker></defs>"
    );
    s
}

fn open(name: &str, w: u32, h: u32, title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"dg-{name}\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-label=\"{title}\">\n<title>{title}</title>\n{}",
        style(name)
    )
}

/// A hand-drawn diagram (docs/guide/img/NAME.svg) with the current stylesheet: its
/// root scoped as `dg-NAME`, its `<style>` and `<defs>` replaced. Its body uses the
/// classes of `RULES`.
pub fn restyle(name: &str, svg: &str) -> Result<String, String> {
    let (Some(a), Some(b)) = (svg.find("<style>"), svg.find("</defs>")) else {
        return Err(format!(
            "img/{name}.svg: needs a <style>...</style><defs>...</defs> block (any content; regen rewrites it)"
        ));
    };
    let rest = &svg[b + "</defs>".len()..];
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let head = &svg[..a];
    let tag_end = head
        .find('>')
        .ok_or(format!("img/{name}.svg: no <svg> tag"))?;
    let tag = &head[..tag_end];
    let class = format!("class=\"dg-{name}\"");
    let tag = match tag.find(" class=\"") {
        Some(i) => {
            let j = tag[i + 8..]
                .find('"')
                .map(|j| i + 8 + j + 1)
                .unwrap_or(tag.len());
            format!("{} {class}{}", &tag[..i], &tag[j..])
        }
        None => tag.replacen("<svg ", &format!("<svg {class} "), 1),
    };
    Ok(format!("{tag}{}{}{rest}", &head[tag_end..], style(name)))
}

fn rect(s: &mut String, class: &str, x: f32, y: f32, w: f32, h: f32) {
    let _ = writeln!(
        s,
        "<rect class=\"{class}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"3\"/>"
    );
}

fn text(s: &mut String, class: &str, x: f32, y: f32, anchor: &str, t: &str) {
    let _ = writeln!(
        s,
        "<text class=\"{class}\" x=\"{x}\" y=\"{y}\" text-anchor=\"{anchor}\">{t}</text>"
    );
}

/// The 64 bits of a value: the f64 view and the tagged view.
pub fn value_bits(prefix: u64) -> String {
    let bit = 10.0_f32; // px per bit
    let x0 = 112.0_f32;
    let w = x0 + 64.0 * bit + 16.0;
    let mut s = open(
        "value-bits",
        w as u32,
        196,
        "The two encodings of a 64-bit djs value",
    );
    // x of the left edge of bit `b` (bit 63 is leftmost).
    let bx = |b: u32| x0 + (63 - b) as f32 * bit;
    let row = |s: &mut String, y: f32, parts: &[(u32, u32, &str, &str, &str)]| {
        for &(hi, lo, class, label, sub) in parts {
            let x = bx(hi);
            let wd = (hi - lo + 1) as f32 * bit;
            rect(s, class, x, y, wd, 34.0);
            text(s, "mono", x + wd / 2.0, y + 15.0, "middle", label);
            text(s, "small", x + wd / 2.0, y + 28.0, "middle", sub);
        }
    };
    text(&mut s, "label", 8.0, 46.0, "start", "Number");
    text(&mut s, "small", 8.0, 60.0, "start", "(any other bits)");
    row(
        &mut s,
        28.0,
        &[
            (63, 63, "box", "s", ""),
            (62, 52, "box", "exponent", "11 bits"),
            (51, 0, "box", "fraction", "52 bits"),
        ],
    );
    text(&mut s, "label", 8.0, 106.0, "start", "Tagged");
    text(
        &mut s,
        "small",
        8.0,
        120.0,
        "start",
        &format!("(top 16 = 0x{prefix:04X})"),
    );
    let pfx = format!("0x{prefix:04X}");
    row(
        &mut s,
        88.0,
        &[
            (63, 48, "tag", &pfx, "quiet NaN, sign set"),
            (47, 32, "tag", "tag", "1–6 immediate, ≥ 8 reference"),
            (
                31,
                0,
                "traced",
                "payload",
                "immediate bits or u32 arena offset",
            ),
        ],
    );
    // Bit ruler.
    for b in [63u32, 48, 32, 0] {
        let x = if b == 0 { bx(0) + bit } else { bx(b) };
        let _ = writeln!(
            s,
            "<line class=\"rule\" x1=\"{x}\" y1=\"16\" x2=\"{x}\" y2=\"26\"/>"
        );
        text(&mut s, "small mono", x, 13.0, "middle", &b.to_string());
    }
    // The JS reader's view: two little-endian u32 words.
    let y = 140.0;
    let mid = bx(31);
    let _ = writeln!(
        s,
        "<path class=\"rule\" fill=\"none\" d=\"M{} {y} v6 H{} v-6\"/>",
        bx(63) + 1.0,
        mid - 2.0
    );
    let _ = writeln!(
        s,
        "<path class=\"rule\" fill=\"none\" d=\"M{} {y} v6 H{} v-6\"/>",
        mid + 2.0,
        bx(0) + bit - 1.0
    );
    text(
        &mut s,
        "small",
        (bx(63) + mid) / 2.0,
        y + 22.0,
        "middle",
        "hi word: <tspan class=\"mono\">hi &gt;&gt;&gt; 16 === 0xFFF8</tspan> means tagged",
    );
    text(
        &mut s,
        "small",
        (mid + bx(0) + bit) / 2.0,
        y + 22.0,
        "middle",
        "lo word: the payload",
    );
    text(
        &mut s,
        "small",
        x0,
        186.0,
        "start",
        "Every NaN is stored as 0x7FF8_0000_0000_0000, so no Number can look like a tagged value.",
    );
    s.push_str("</svg>\n");
    s
}

/// A cell kind's bytes, 8 per row: the header, the fixed fields, the tail.
pub fn cell(l: &CellLayout) -> String {
    let byte = 64.0_f32;
    let x0 = 56.0_f32;
    let row_h = 44.0_f32;
    let y0 = 30.0_f32;
    // Fields from `min_size` on may be optional (D1980): a cell can end before them.
    let fields_end = l
        .fields
        .iter()
        .map(|f| f.offset + f.ty.size())
        .max()
        .unwrap_or(8);
    let fixed_end = l
        .tail
        .map(|t| t.offset)
        .unwrap_or(fields_end.max(l.min_size));
    let optional = |off: u32| l.tail.is_none() && off >= l.min_size;
    let any_optional = l.fields.iter().any(|f| optional(f.offset));
    let fixed_rows = fixed_end.div_ceil(8);
    let rows = fixed_rows + if l.tail.is_some() { 1 } else { 0 };
    let h = y0 + rows as f32 * row_h + 52.0;
    let w = x0 + 8.0 * byte + 12.0;
    let mut s = open(
        &format!("cell-{}", l.name),
        w as u32,
        h as u32,
        &format!("Layout of the {} cell", l.name),
    );
    for b in 0..8 {
        text(
            &mut s,
            "small mono",
            x0 + b as f32 * byte + byte / 2.0,
            y0 - 8.0,
            "middle",
            &format!("+{b}"),
        );
    }
    let place = |s: &mut String, off: u32, size: u32, class: &str, name: &str, ty: &str| {
        // Fields are naturally aligned and at most 8 bytes, so they never cross a row.
        let r = off / 8;
        let x = x0 + (off % 8) as f32 * byte;
        let y = y0 + r as f32 * row_h;
        let wd = size as f32 * byte;
        rect(s, class, x + 1.0, y + 1.0, wd - 2.0, row_h - 4.0);
        text(s, "mono", x + wd / 2.0, y + 18.0, "middle", name);
        text(s, "small", x + wd / 2.0, y + 32.0, "middle", ty);
    };
    for r in 0..rows {
        text(
            &mut s,
            "small mono",
            x0 - 8.0,
            y0 + r as f32 * row_h + 25.0,
            "end",
            &(r * 8).to_string(),
        );
    }
    // The header (the same for every cell kind).
    for (off, size, name) in [
        (0, 1, "KIND"),
        (1, 1, "GC"),
        (2, 2, "FLAGS"),
        (4, 4, "SIZE"),
    ] {
        let ty = match size {
            1 => "u8",
            2 => "u16",
            _ => "u32",
        };
        place(&mut s, off, size, "hdr", name, ty);
    }
    let mut used = vec![false; (fixed_rows * 8) as usize];
    used[..8].iter_mut().for_each(|u| *u = true);
    for f in l.fields {
        let class = match (f.ty.is_traced(), optional(f.offset)) {
            (true, false) => "traced",
            (true, true) => "traced opt",
            (false, false) => "box",
            (false, true) => "box opt",
        };
        place(&mut s, f.offset, f.ty.size(), class, f.name, export(f.ty));
        for b in f.offset..f.offset + f.ty.size() {
            used[b as usize] = true;
        }
    }
    for (b, u) in used.iter().enumerate() {
        if !u {
            let x = x0 + (b % 8) as f32 * byte;
            let y = y0 + (b / 8) as f32 * row_h;
            rect(&mut s, "pad", x + 1.0, y + 1.0, byte - 2.0, row_h - 4.0);
        }
    }
    if let Some(t) = l.tail {
        let y = y0 + fixed_rows as f32 * row_h;
        let traced = t.elem.fields.iter().any(|f| f.ty.is_traced());
        rect(
            &mut s,
            if traced { "traced" } else { "tail" },
            x0 + 1.0,
            y + 1.0,
            8.0 * byte - 2.0,
            row_h - 4.0,
        );
        let elem = if t.elem.fields.len() == 1 && t.elem.fields[0].name.is_empty() {
            export(t.elem.fields[0].ty).to_string()
        } else {
            t.elem.name.to_string()
        };
        text(
            &mut s,
            "mono",
            x0 + 4.0 * byte,
            y + 18.0,
            "middle",
            &format!("{}: {elem} × n", t.name),
        );
        text(
            &mut s,
            "small",
            x0 + 4.0 * byte,
            y + 32.0,
            "middle",
            "variable-length tail; n follows from SIZE",
        );
    }
    // Legend.
    let ly = h - 16.0;
    let mut lx = x0;
    let mut legend = vec![
        ("hdr", "header"),
        ("traced", "traced by the GC (Value, Ref)"),
        ("box", "plain data"),
        ("pad", "padding"),
    ];
    if any_optional {
        legend.push(("box opt", "optional"));
    }
    for (class, label) in legend {
        rect(&mut s, class, lx, ly - 10.0, 14.0, 12.0);
        text(&mut s, "small", lx + 20.0, ly, "start", label);
        lx += 40.0 + label.len() as f32 * 5.6;
    }
    s.push_str("</svg>\n");
    s
}

fn export(t: djs_core::heap::layout::FieldTy) -> &'static str {
    t.export_name()
}
