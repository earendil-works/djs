//! Diagrams drawn from the engine's own layout data. Plain SVG strings with an embedded
//! stylesheet (light and dark), so the same file works inline in the site and as an
//! `<img>` on GitHub.

use std::fmt::Write;

use djs_core::heap::layout::CellLayout;

const STYLE: &str = r#"<style>
  text { font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 12px; fill: #1f2328; }
  .mono { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; }
  .small { font-size: 10px; fill: #59636e; }
  .label { font-weight: 600; }
  .box { fill: #f6f8fa; stroke: #8c959f; }
  .hdr { fill: #ddf4ff; stroke: #54aeff; }
  .traced { fill: #fff1e5; stroke: #fb8f44; }
  .tag { fill: #fbefff; stroke: #c297ff; }
  .pad { fill: url(#hatch); stroke: #d0d7de; }
  .tail { fill: #f6f8fa; stroke: #8c959f; stroke-dasharray: 4 3; }
  .opt { stroke-dasharray: 3 2; }
  .rule { stroke: #8c959f; }
  .hatch { stroke: #d0d7de; }
  @media (prefers-color-scheme: dark) {
    text { fill: #e6edf3; }
    .small { fill: #9198a1; }
    .box, .tail { fill: #161b22; stroke: #6e7681; }
    .hdr { fill: #0c2d6b; stroke: #388bfd; }
    .traced { fill: #3d1d00; stroke: #db6d28; }
    .tag { fill: #2b1a4a; stroke: #a371f7; }
    .pad { stroke: #30363d; }
    .rule { stroke: #6e7681; }
    .hatch { stroke: #30363d; }
  }
</style>
<defs><pattern id="hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="6" class="hatch"/></pattern></defs>
"#;

fn open(w: u32, h: u32, title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-label=\"{title}\">\n<title>{title}</title>\n{STYLE}"
    )
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
    let mut s = open(w as u32, 196, "The two encodings of a 64-bit djs value");
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
