//! Charts of measured numbers (the benchmarks page): plain SVG with the diagrams'
//! stylesheet (svg.rs), so they follow the site's colors in light and dark mode and work
//! as an `<img>` too. Each carries a `<title>` and `<desc>`; the page keeps the table
//! with the exact numbers next to it. Three shapes cover the page: grouped columns
//! (a value per group and series), horizontal bars in side-by-side panels (a value per
//! row and panel) and ratios around a parity line.

use std::fmt::Write;

use crate::svg;

/// Series colors: Elanor's data-visualization palette, the same in light and dark mode.
const SERIES: [&str; 10] = [
    "100", "200", "300", "400", "500", "600", "700", "800", "900", "1000",
];

fn extra_rules() -> Vec<(String, String)> {
    let mut r: Vec<(String, String)> = vec![
        (
            ".grid".into(),
            "stroke: $[border-secondary]; stroke-width: 1;".into(),
        ),
        (
            ".axis".into(),
            "stroke: $[border-primary]; stroke-width: 1;".into(),
        ),
        (
            ".parity".into(),
            "stroke: $[text-secondary]; stroke-width: 1.2; stroke-dasharray: 4 3;".into(),
        ),
        (
            ".faster".into(),
            "fill: $[background-fill-primary-positive];".into(),
        ),
        (
            ".slower".into(),
            "fill: $[background-fill-primary-negative];".into(),
        ),
    ];
    for (i, c) in SERIES.iter().enumerate() {
        r.push((
            format!(".s{i}"),
            format!("fill: $[background-fill-data-viz-{c}];"),
        ));
    }
    r
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('`', "")
}

/// Approximate text width: the charts lay out labels without a font to measure them by.
fn width(t: &str, px: f32) -> f32 {
    t.chars().count() as f32 * px * 0.56
}

/// The same for the monospace font, a little wider.
fn mono_width(t: &str) -> f32 {
    t.chars().count() as f32 * 11.0 * 0.62
}

fn open(m: &Meta, w: f32, h: f32) -> String {
    let Meta { name, title, desc } = *m;
    let extra = extra_rules();
    let extra: Vec<(&str, &str)> = extra
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"dg-{name}\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-labelledby=\"{name}-t {name}-d\">\n<title id=\"{name}-t\">{}</title>\n<desc id=\"{name}-d\">{}</desc>\n{}",
        esc(title),
        esc(desc),
        svg::style_with(name, &extra)
    )
}

fn text(s: &mut String, class: &str, x: f32, y: f32, anchor: &str, t: &str) {
    let _ = writeln!(
        s,
        "<text class=\"{class}\" x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"{anchor}\">{}</text>",
        esc(t)
    );
}

fn bar(s: &mut String, class: &str, x: f32, y: f32, w: f32, h: f32) {
    let _ = writeln!(
        s,
        "<rect class=\"{class}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>",
        w.max(0.5),
        h.max(0.5)
    );
}

fn line(s: &mut String, class: &str, x1: f32, y1: f32, x2: f32, y2: f32) {
    let _ = writeln!(
        s,
        "<line class=\"{class}\" x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\"/>"
    );
}

/// A number as an axis or bar label: three significant digits at most, no trailing zeros.
pub fn short(v: f64) -> String {
    let s = if v == 0.0 {
        "0".to_string()
    } else if v.abs() >= 1000.0 {
        let s = format!("{:.0}", v);
        // Thousands separators.
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        return out;
    } else if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else if v.abs() >= 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    };
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// A value axis: linear from 0, or logarithmic between 1-2-5 bounds.
struct Axis {
    log: bool,
    lo: f64,
    hi: f64,
    ticks: Vec<f64>,
}

impl Axis {
    fn new(values: impl Iterator<Item = f64>, log: bool) -> Axis {
        let vs: Vec<f64> = values
            .filter(|v| v.is_finite() && (!log || *v > 0.0))
            .collect();
        let max = vs.iter().cloned().fold(f64::MIN, f64::max).max(1e-9);
        if !log {
            let step = nice_step(max / 4.0);
            let hi = (max / step).ceil() * step;
            let n = (hi / step).round() as usize;
            let ticks = (0..=n).map(|i| i as f64 * step).collect();
            return Axis {
                log,
                lo: 0.0,
                hi,
                ticks,
            };
        }
        let min = vs.iter().cloned().fold(f64::MAX, f64::min).min(max);
        // A step below the smallest value, so that its bar has a length to compare.
        let lo = nice_below(min / 1.8);
        let hi = nice_above(max);
        let mut ticks = Vec::new();
        let mut decade = 10f64.powf(lo.log10().floor());
        while decade <= hi * 1.0001 {
            for m in [1.0, 2.0, 5.0] {
                let t = decade * m;
                if t >= lo * 0.9999 && t <= hi * 1.0001 {
                    ticks.push(t);
                }
            }
            decade *= 10.0;
        }
        Axis { log, lo, hi, ticks }
    }

    /// The ticks that fit along `span` pixels with their labels apart: on a logarithmic
    /// axis the 2s go first, then the 5s, then every other power of ten.
    fn fit(&self, span: f32, label: impl Fn(f64) -> f32) -> Vec<f64> {
        let fits = |ts: &[f64]| {
            ts.windows(2).all(|w| {
                (self.at(w[1]) - self.at(w[0])) * span >= (label(w[0]) + label(w[1])) / 2.0 + 8.0
            })
        };
        if !self.log || fits(&self.ticks) {
            return self.ticks.clone();
        }
        // Mantissa of a tick: 1, 2 or 5.
        let m = |t: &f64| (t / 10f64.powf((t.log10() + 1e-9).floor())).round();
        let no2: Vec<f64> = self.ticks.iter().cloned().filter(|t| m(t) != 2.0).collect();
        if fits(&no2) {
            return no2;
        }
        let ones: Vec<f64> = no2.into_iter().filter(|t| m(t) == 1.0).collect();
        if fits(&ones) {
            return ones;
        }
        ones.into_iter().step_by(2).collect()
    }

    /// 0..1 along the axis.
    fn at(&self, v: f64) -> f32 {
        let f = if self.log {
            (v.max(self.lo).ln() - self.lo.ln()) / (self.hi.ln() - self.lo.ln())
        } else {
            (v - self.lo) / (self.hi - self.lo)
        };
        f.clamp(0.0, 1.0) as f32
    }
}

fn nice_step(raw: f64) -> f64 {
    let p = 10f64.powf(raw.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if p * m >= raw {
            return p * m;
        }
    }
    p * 10.0
}

fn nice_above(v: f64) -> f64 {
    let p = 10f64.powf(v.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if p * m >= v * 0.9999 {
            return p * m;
        }
    }
    p * 10.0
}

fn nice_below(v: f64) -> f64 {
    let p = 10f64.powf(v.log10().floor());
    for m in [5.0, 2.0, 1.0] {
        if p * m <= v * 1.0001 {
            return p * m;
        }
    }
    p
}

/// What every chart has: its name (the file's, and the root of its CSS scope), its title
/// and its description for screen readers.
pub struct Meta<'a> {
    pub name: &'a str,
    pub title: &'a str,
    pub desc: &'a str,
}

/// One series of a chart: its legend label and palette color.
pub struct Series<'a> {
    pub label: &'a str,
    pub color: usize,
}

fn legend(s: &mut String, series: &[Series], x0: f32, y: f32, max_w: f32) -> f32 {
    let mut x = x0;
    let mut y = y;
    for se in series {
        let w = 16.0 + width(se.label, 12.0) + 16.0;
        if x + w > x0 + max_w && x > x0 {
            x = x0;
            y += 18.0;
        }
        bar(s, &format!("s{}", se.color), x, y - 9.0, 10.0, 10.0);
        text(s, "", x + 15.0, y, "start", se.label);
        x += w;
    }
    y
}

const W: f32 = 720.0;

/// Grouped columns: `values[group][series]` (None: no column) on a value axis, groups
/// along the bottom, a legend on top.
pub fn columns(
    m: &Meta,
    unit: &str,
    groups: &[&str],
    series: &[Series],
    values: &[Vec<Option<f64>>],
    log: bool,
) -> String {
    let axis = Axis::new(values.iter().flatten().flatten().cloned(), log);
    let mut body = String::new();
    let left = 52.0;
    let right = 8.0;
    let top = legend(&mut body, series, left, 14.0, W - left - right) + 16.0;
    let plot_h = 200.0;
    let bottom = top + plot_h;
    let plot_w = W - left - right;
    let gw = plot_w / groups.len() as f32;
    let col = ((gw - 10.0) / series.len() as f32).min(14.0);
    for t in &axis.fit(plot_h, |_| 12.0) {
        let y = bottom - axis.at(*t) * plot_h;
        line(&mut body, "grid", left, y, W - right, y);
        text(&mut body, "small", left - 6.0, y + 3.5, "end", &short(*t));
    }
    text(&mut body, "small", left - 6.0, top - 8.0, "end", unit);
    for (g, name) in groups.iter().enumerate() {
        let x0 = left + g as f32 * gw + (gw - col * series.len() as f32) / 2.0;
        for (i, se) in series.iter().enumerate() {
            if let Some(v) = values[g][i] {
                let h = axis.at(v) * plot_h;
                bar(
                    &mut body,
                    &format!("s{}", se.color),
                    x0 + i as f32 * col,
                    bottom - h,
                    col - 1.0,
                    h,
                );
            }
        }
        text(
            &mut body,
            "",
            left + (g as f32 + 0.5) * gw,
            bottom + 16.0,
            "middle",
            name,
        );
    }
    line(&mut body, "axis", left, bottom, W - right, bottom);
    let h = bottom + 26.0;
    format!("{}{body}</svg>\n", open(m, W, h))
}

/// One panel of [`bars`]: a title over a column of bars, one per row.
pub struct Panel<'a> {
    pub title: &'a str,
    pub values: Vec<Option<f64>>,
    pub log: bool,
    pub format: fn(f64) -> String,
}

/// Horizontal bars: a row per entry of `rows` (label, palette color), a panel per measure
/// side by side, each bar labeled with its value.
pub fn bars(m: &Meta, rows: &[(&str, usize)], panels: &[Panel]) -> String {
    let label_w = rows.iter().map(|(l, _)| width(l, 12.0)).fold(0.0, f32::max) + 12.0;
    let gap = 20.0;
    let pw = (W - label_w - gap * (panels.len() as f32 - 1.0)) / panels.len() as f32;
    let row_h = 18.0;
    let top = 34.0;
    let mut body = String::new();
    for (r, (label, _)) in rows.iter().enumerate() {
        text(
            &mut body,
            "",
            label_w - 8.0,
            top + r as f32 * row_h + 12.5,
            "end",
            label,
        );
    }
    let bottom = top + rows.len() as f32 * row_h;
    for (p, panel) in panels.iter().enumerate() {
        let x0 = label_w + p as f32 * (pw + gap);
        // Room for the value after the longest bar.
        let span = pw - 46.0;
        let axis = Axis::new(panel.values.iter().flatten().cloned(), panel.log);
        text(&mut body, "label", x0, 14.0, "start", panel.title);
        for t in &axis.fit(span, |t| width(&short(t), 10.5)) {
            let x = x0 + axis.at(*t) * span;
            line(&mut body, "grid", x, top - 4.0, x, bottom);
            text(&mut body, "small", x, top - 8.0, "middle", &short(*t));
        }
        for (r, (_, color)) in rows.iter().enumerate() {
            let y = top + r as f32 * row_h + 3.0;
            match panel.values[r] {
                Some(v) => {
                    let w = axis.at(v) * span;
                    bar(&mut body, &format!("s{color}"), x0, y, w, row_h - 6.0);
                    text(
                        &mut body,
                        "small",
                        x0 + w + 4.0,
                        y + 10.0,
                        "start",
                        &(panel.format)(v),
                    );
                }
                None => text(&mut body, "small", x0 + 4.0, y + 10.0, "start", "none"),
            }
        }
        line(&mut body, "axis", x0, top - 4.0, x0, bottom);
    }
    let h = bottom + 8.0;
    format!("{}{body}</svg>\n", open(m, W, h))
}

/// Ratios on a logarithmic axis centered on a parity line at 1: a bar from the line to
/// each row's value, `faster` below 1 and `slower` above.
pub fn ratios(m: &Meta, rows: &[(&str, Option<f64>)], below: &str, above: &str) -> String {
    let label_w = rows
        .iter()
        .map(|(l, _)| mono_width(&esc(l)))
        .fold(0.0, f32::max)
        + 12.0;
    let max = rows
        .iter()
        .filter_map(|(_, v)| *v)
        .map(|v| v.ln().abs())
        .fold(0.0f64, f64::max)
        .exp();
    let d = [1.5, 2.0, 3.0, 4.0, 5.0, 10.0]
        .into_iter()
        .find(|d| *d >= max * 1.05)
        .unwrap_or(max * 1.1);
    let axis = Axis {
        log: true,
        lo: 1.0 / d,
        hi: d,
        ticks: Vec::new(),
    };
    let mut ticks = vec![1.0];
    let mut t = 2.0;
    while t <= d * 1.0001 {
        ticks.push(t);
        ticks.push(1.0 / t);
        t *= 2.0;
    }
    let row_h = 17.0;
    let top = 40.0;
    let x0 = label_w;
    let span = W - x0 - 40.0;
    let bottom = top + rows.len() as f32 * row_h;
    let mut body = String::new();
    let mid = x0 + axis.at(1.0) * span;
    text(
        &mut body,
        "small",
        mid - 8.0,
        14.0,
        "end",
        &format!("← {below}"),
    );
    text(
        &mut body,
        "small",
        mid + 8.0,
        14.0,
        "start",
        &format!("{above} →"),
    );
    for t in &ticks {
        let x = x0 + axis.at(*t) * span;
        if *t != 1.0 {
            line(&mut body, "grid", x, top - 4.0, x, bottom);
        }
        text(
            &mut body,
            "small",
            x,
            top - 9.0,
            "middle",
            &format!("{}×", short(*t)),
        );
    }
    for (r, (label, v)) in rows.iter().enumerate() {
        let y = top + r as f32 * row_h;
        text(&mut body, "mono", label_w - 8.0, y + 12.0, "end", label);
        match v {
            Some(v) => {
                let x = x0 + axis.at(*v) * span;
                let (a, b) = if x < mid { (x, mid) } else { (mid, x) };
                bar(
                    &mut body,
                    if *v < 1.0 { "faster" } else { "slower" },
                    a,
                    y + 3.0,
                    b - a,
                    row_h - 6.0,
                );
                let (tx, anchor) = if *v < 1.0 {
                    (a - 4.0, "end")
                } else {
                    (b + 4.0, "start")
                };
                text(&mut body, "small", tx, y + 12.0, anchor, &format!("{v:.2}"));
            }
            None => text(
                &mut body,
                "small",
                mid + 4.0,
                y + 12.0,
                "start",
                "too short to compare",
            ),
        }
    }
    line(&mut body, "parity", mid, top - 4.0, mid, bottom);
    let h = bottom + 8.0;
    format!("{}{body}</svg>\n", open(m, W, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes() {
        let a = Axis::new([0.21, 29.3].into_iter(), true);
        assert_eq!((a.lo, a.hi), (0.1, 50.0));
        assert_eq!(a.ticks.len(), 9);
        assert_eq!(a.fit(100.0, |_| 20.0), vec![0.1, 1.0, 10.0]);
        assert_eq!(a.fit(1000.0, |_| 20.0).len(), 9);
        let a = Axis::new([0.4, 3.0].into_iter(), true);
        assert_eq!(a.ticks, vec![0.2, 0.5, 1.0, 2.0, 5.0]);
        let a = Axis::new([3.0, 17.0].into_iter(), false);
        assert_eq!((a.lo, a.hi), (0.0, 20.0));
        assert_eq!(short(0.123456), "0.123");
        assert_eq!(short(12.5), "12.5");
        assert_eq!(short(24593.0), "24,593");
        assert_eq!(short(2.0), "2");
    }

    #[test]
    fn charts_are_labeled() {
        let s = columns(
            &Meta {
                name: "t",
                title: "Run <time>",
                desc: "desc",
            },
            "ms",
            &["a", "b"],
            &[
                Series {
                    label: "djs",
                    color: 0,
                },
                Series {
                    label: "x",
                    color: 1,
                },
            ],
            &[vec![Some(1.0), Some(2.0)], vec![Some(0.5), None]],
            true,
        );
        assert!(s.contains("<title id=\"t-t\">Run &lt;time&gt;</title>"));
        assert!(s.contains("aria-labelledby=\"t-t t-d\""));
        assert!(s.ends_with("</svg>\n"));
        let r = ratios(
            &Meta {
                name: "r",
                title: "t",
                desc: "d",
            },
            &[("`a`", Some(0.5)), ("b", Some(2.8)), ("c", None)],
            "faster",
            "slower",
        );
        assert!(r.contains("class=\"faster\"") && r.contains("class=\"slower\""));
        assert!(r.contains(">a</text>"));
    }
}
