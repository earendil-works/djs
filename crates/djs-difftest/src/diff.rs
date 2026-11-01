//! A small line diff for failure reports (LCS; transcripts are short).

/// A line diff of `expected` → `actual`: ` ` common, `-` only expected, `+` only actual.
/// Runs of common lines longer than `2 * context` are elided.
pub fn line_diff(expected: &str, actual: &str, context: usize) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    // Very long transcripts: skip the quadratic table and show the first difference.
    if a.len() * b.len() > 4_000_000 {
        let i = a
            .iter()
            .zip(&b)
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len()));
        return format!(
            "(long transcripts; first difference at line {})\n-{}\n+{}\n",
            i + 1,
            a.get(i).unwrap_or(&"<end>"),
            b.get(i).unwrap_or(&"<end>")
        );
    }
    // lcs[i][j] = LCS length of a[i..] and b[j..]
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut ops: Vec<(char, &str)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            ops.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            ops.push(('+', b[j]));
            j += 1;
        } else {
            ops.push(('-', a[i]));
            i += 1;
        }
    }
    let mut out = String::new();
    let mut k = 0;
    while k < ops.len() {
        if ops[k].0 != ' ' {
            out.push_str(&format!("{}{}\n", ops[k].0, ops[k].1));
            k += 1;
            continue;
        }
        let start = k;
        while k < ops.len() && ops[k].0 == ' ' {
            k += 1;
        }
        let run = &ops[start..k];
        let lead = if start == 0 { 0 } else { context };
        let trail = if k == ops.len() { 0 } else { context };
        if run.len() > lead + trail + 1 {
            for (_, l) in &run[..lead] {
                out.push_str(&format!(" {l}\n"));
            }
            out.push_str(&format!(
                "  ... {} unchanged lines\n",
                run.len() - lead - trail
            ));
            for (_, l) in &run[run.len() - trail..] {
                out.push_str(&format!(" {l}\n"));
            }
        } else {
            for (_, l) in run {
                out.push_str(&format!(" {l}\n"));
            }
        }
    }
    out
}

/// A one-line description of the first difference, for the failures file.
pub fn first_difference(expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    let i = a
        .iter()
        .zip(&b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let show = |l: Option<&&str>| match l {
        Some(l) => format!("`{l}`"),
        None => "end".to_string(),
    };
    format!(
        "line {}: expected {}, got {}",
        i + 1,
        show(a.get(i)),
        show(b.get(i))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs() {
        let d = line_diff("a\nb\nc\n", "a\nx\nc\n", 3);
        assert_eq!(d, " a\n+x\n-b\n c\n");
        let exp: String = (0..20).map(|i| format!("{i}\n")).collect();
        let act = exp.replace("10\n", "ten\n");
        let d = line_diff(&exp, &act, 2);
        assert!(
            d.starts_with(
                "  ... 8 unchanged lines\n 8\n 9\n+ten\n-10\n 11\n 12\n  ... 7 unchanged lines\n"
            ),
            "{d}"
        );
        assert_eq!(line_diff("a\n", "a\n", 3), " a\n");
    }

    #[test]
    fn first_diff() {
        assert_eq!(
            first_difference("a\nb\n", "a\nc\n"),
            "line 2: expected `b`, got `c`"
        );
        assert_eq!(
            first_difference("a\n", "a\nb\n"),
            "line 2: expected end, got `b`"
        );
    }
}
