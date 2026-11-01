//! Behavior tests for the regex engine.

extern crate std;

use super::*;
use std::string::String;
use std::vec::Vec;

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn re(pattern: &str, flags: &str) -> Program {
    compile(&u16s(pattern), Flags::parse(&u16s(flags)).unwrap())
        .unwrap_or_else(|e| panic!("/{pattern}/{flags}: {}", e.message()))
}

fn syntax_error(pattern: &str, flags: &str) -> String {
    match compile(&u16s(pattern), Flags::parse(&u16s(flags)).unwrap()) {
        Ok(_) => panic!("/{pattern}/{flags} compiled"),
        Err(e) => e.message().into(),
    }
}

/// Runs to completion; captures as strings (None = undefined).
fn exec_str(prog: &Program, input: &str, start: usize) -> Option<Vec<Option<String>>> {
    let text = u16s(input);
    let mut fuel = u64::MAX;
    match exec(
        prog,
        Input::Utf16(&text),
        start,
        &mut fuel,
        ExecLimits::default(),
    ) {
        ExecResult::Match(c) => Some(
            (0..c.len())
                .map(|i| {
                    c.get(i)
                        .map(|(s, e)| String::from_utf16(&text[s..e]).unwrap())
                })
                .collect(),
        ),
        ExecResult::NoMatch => None,
        _ => panic!("limit"),
    }
}

fn m(pattern: &str, flags: &str, input: &str) -> Option<Vec<Option<String>>> {
    exec_str(&re(pattern, flags), input, 0)
}

fn first(pattern: &str, flags: &str, input: &str) -> Option<String> {
    m(pattern, flags, input).map(|c| c[0].clone().unwrap())
}

fn caps(v: &[Option<&str>]) -> Option<Vec<Option<String>>> {
    Some(v.iter().map(|s| s.map(String::from)).collect())
}

#[test]
fn basics() {
    assert_eq!(first("abc", "", "xxabcxx"), Some("abc".into()));
    assert_eq!(first("a|b", "", "xb"), Some("b".into()));
    assert_eq!(first("a*", "", "aaab"), Some("aaa".into()));
    assert_eq!(first("a*?", "", "aaab"), Some("".into()));
    assert_eq!(first("a+?", "", "aaab"), Some("a".into()));
    assert_eq!(first("a{2,3}", "", "aaaa"), Some("aaa".into()));
    assert_eq!(first("a{2,3}?", "", "aaaa"), Some("aa".into()));
    assert_eq!(first("a{2}", "", "aaaa"), Some("aa".into()));
    assert_eq!(first("a{2,}", "", "aaaa"), Some("aaaa".into()));
    assert_eq!(first("^abc$", "", "abc"), Some("abc".into()));
    assert_eq!(first("^b", "m", "a\nb"), Some("b".into()));
    assert_eq!(first("^b", "", "a\nb"), None);
    assert_eq!(first("a.c", "", "a\nc"), None);
    assert_eq!(first("a.c", "s", "a\nc"), Some("a\nc".into()));
    assert_eq!(first("[a-c]+", "", "xxbcaz"), Some("bca".into()));
    assert_eq!(first("[^a-c]+", "", "abxyc"), Some("xy".into()));
    assert_eq!(first("\\d+", "", "ab123c"), Some("123".into()));
    assert_eq!(first("\\bfoo\\b", "", "a foo b"), Some("foo".into()));
    assert_eq!(first("\\Boo", "", "foo"), Some("oo".into()));
    assert_eq!(first("ABC", "i", "xabcx"), Some("abc".into()));
    assert_eq!(first("[a-z]+", "i", "XYZ"), Some("XYZ".into()));
}

#[test]
fn groups_and_backrefs() {
    assert_eq!(m("(a)(b)?", "", "a"), caps(&[Some("a"), Some("a"), None]));
    assert_eq!(
        m("(a|ab)(c|bcd)(d*)", "", "abcd"),
        caps(&[Some("abcd"), Some("a"), Some("bcd"), Some("")])
    );
    assert_eq!(
        m("(z)((a+)?(b+)?(c))*", "", "zaacbbbcac"),
        caps(&[
            Some("zaacbbbcac"),
            Some("z"),
            Some("ac"),
            Some("a"),
            None,
            Some("c")
        ])
    );
    assert_eq!(m("(a*)*", "", "b"), caps(&[Some(""), None]));
    assert_eq!(m("(a*)+", "", "b"), caps(&[Some(""), Some("")]));
    assert_eq!(
        m("(.*?)a(?!(a+)b\\2c)\\2(.*)", "", "baaabaac"),
        caps(&[Some("baaabaac"), Some("ba"), None, Some("abaac")])
    );
    assert_eq!(m("(a)\\1", "", "aa"), caps(&[Some("aa"), Some("a")]));
    assert_eq!(m("\\1(a)", "", "aa"), caps(&[Some("a"), Some("a")]));
    assert_eq!(m("(?<x>a)\\k<x>", "", "aa"), caps(&[Some("aa"), Some("a")]));
    assert_eq!(m("(a)\\1", "i", "aA"), caps(&[Some("aA"), Some("a")]));
    assert_eq!(
        m("(?:(?<a>x)|(?<a>y))\\k<a>", "", "yy"),
        caps(&[Some("yy"), None, Some("y")])
    );
    let p = re("(?<year>\\d{4})-(?<m>\\d\\d)", "");
    assert_eq!(p.capture_count(), 3);
    assert_eq!(p.group_name(1).map(|n| n.len()), Some(4));
    assert!(p.group_name(0).is_none());
}

#[test]
fn lookaround() {
    assert_eq!(first("a(?=b)", "", "acab"), Some("a".into()));
    assert_eq!(
        m("(?=(a+))a*b\\1", "", "baaabac"),
        caps(&[Some("aba"), Some("a")])
    );
    assert_eq!(first("a(?!b)", "", "abac"), Some("a".into()));
    assert_eq!(first("(?<=\\$)\\d+", "", "cost $42"), Some("42".into()));
    assert_eq!(first("(?<!\\$)\\b\\d+", "", "$4 5"), Some("5".into()));
    assert_eq!(
        m("(?<=(\\d+)(\\d+))$", "", "1053"),
        caps(&[Some(""), Some("1"), Some("053")])
    );
    assert_eq!(m("(?<=\\1(a))b", "", "aab"), caps(&[Some("b"), Some("a")]));
    assert_eq!(m("(?=(a))?", "", "a"), caps(&[Some(""), None]));
}

#[test]
fn unicode_mode() {
    assert_eq!(first("^.$", "", "😀"), None);
    assert_eq!(first("^.$", "u", "😀"), Some("😀".into()));
    assert_eq!(first("\\u{1F600}", "u", "x😀"), Some("😀".into()));
    assert_eq!(first("[😀]", "u", "😀"), Some("😀".into()));
    assert_eq!(first("\\u{212A}", "ui", "k"), Some("k".into()));
    assert_eq!(first("\\u{212A}", "i", "k"), None);
    assert_eq!(first("\\w", "ui", "\u{17F}"), Some("\u{17F}".into()));
    assert_eq!(first("\\W", "ui", "\u{17F}"), None);
    assert_eq!(first("[\\W]", "ui", "S"), None);
}

#[test]
fn annex_b() {
    assert_eq!(
        first("{{(\\w+)}}", "", "a {{name}} b"),
        Some("{{name}}".into())
    );
    assert_eq!(first("a{,5}", "", "a{,5}"), Some("a{,5}".into()));
    assert_eq!(first("]", "", "a]"), Some("]".into()));
    assert_eq!(first("\\_", "", "_"), Some("_".into()));
    assert_eq!(first("[\\w-.]+", "", "a-b.c"), Some("a-b.c".into()));
    assert_eq!(first("\\101", "", "A"), Some("A".into()));
    assert_eq!(first("\\8", "", "8"), Some("8".into()));
    assert_eq!(first("\\c", "", "\\c"), Some("\\c".into()));
    assert_eq!(first("[\\c1]", "", "\u{11}"), Some("\u{11}".into()));
    assert_eq!(first("\\u{3}", "", "uuu"), Some("uuu".into()));
    assert_eq!(first("\\k", "", "k"), Some("k".into()));
    assert_eq!(first("(?=a)*a", "", "a"), Some("a".into()));
}

#[test]
fn class_sets() {
    assert_eq!(first("[\\w&&\\d]+", "v", "ab12"), Some("12".into()));
    assert_eq!(first("[\\w--\\d]+", "v", "12ab"), Some("ab".into()));
    assert_eq!(first("[[a-z]--[aeiou]]+", "v", "aabcd"), Some("bcd".into()));
    assert_eq!(first("[\\q{abc|d}x]", "v", "zabc"), Some("abc".into()));
    assert_eq!(first("^[\\q{}]$", "v", ""), Some("".into()));
    assert_eq!(first("[\\q{AB}]", "vi", "ab"), Some("ab".into()));
}

#[test]
fn modifiers() {
    assert_eq!(first("a(?i:b)c", "", "aBc"), Some("aBc".into()));
    assert_eq!(first("a(?i:b)c", "", "aBC"), None);
    assert_eq!(first("(?-i:a)b", "i", "aB"), Some("aB".into()));
    assert_eq!(first("(?-i:a)b", "i", "AB"), None);
    assert_eq!(first("(?s:.)", "", "\n"), Some("\n".into()));
    assert_eq!(first("(?m:^b)", "", "a\nb"), Some("b".into()));
}

#[test]
fn sticky_and_start() {
    let p = re("a", "y");
    assert_eq!(exec_str(&p, "ba", 0), None);
    assert_eq!(exec_str(&p, "ba", 1), caps(&[Some("a")]));
    let p = re("a", "g");
    assert_eq!(exec_str(&p, "aba", 1), caps(&[Some("a")]));
    assert_eq!(exec_str(&p, "aba", 4), None);
    // Starting inside a surrogate pair in unicode mode starts at the pair.
    let p = re(".", "u");
    assert_eq!(exec_str(&p, "😀", 1), caps(&[Some("😀")]));
}

#[test]
fn syntax_errors() {
    for (p, f, msg) in [
        ("(", "", "expecting ')'"),
        ("\\", "", "unexpected end"),
        ("a**", "", "nothing to repeat"),
        ("[b-a]", "", "invalid class range"),
        ("(?<a>x)(?<a>y)", "", "duplicate group name"),
        ("\\k<a>", "u", "group name not defined"),
        ("(?ii:a)", "", "duplicate modifier: 'i'"),
        ("[\\w-a]", "u", "invalid class range"),
        ("a{2,1}", "", "invalid repetition count"),
        (")", "", "extraneous characters at the end"),
        ("(?<=a)*", "", "nothing to repeat"),
        ("(?=a)*", "u", "nothing to repeat"),
        (
            "\\8",
            "u",
            "back reference out of range in regular expression",
        ),
        (
            "\\q{a}",
            "v",
            "invalid escape sequence in regular expression",
        ),
        ("{", "u", "syntax error"),
        ("(?x)", "", "invalid group"),
        ("[", "", "unexpected end"),
        ("\\c", "u", "invalid escape sequence in regular expression"),
        (
            "[a&&&b]",
            "v",
            "invalid class set operation in regular expression",
        ),
        (
            "[(]",
            "v",
            "invalid character in class in regular expression",
        ),
        (
            "[^\\q{ab}]",
            "v",
            "negated character class with strings in regular expression debugger eval code",
        ),
        ("(?<a>x)(?:y|(?<a>z))", "", "duplicate group name"),
    ] {
        assert_eq!(syntax_error(p, f), msg, "/{p}/{f}");
    }
    assert!(Flags::parse(&u16s("gg")).is_err());
    assert!(Flags::parse(&u16s("uv")).is_err());
    assert!(Flags::parse(&u16s("z")).is_err());
    assert!(Flags::parse(&u16s("dgimsuy")).is_ok());
}

/// Case-insensitive equivalence of related characters, against qjs
/// (tests/data/regexp/case.txt, `eq` lines).
#[test]
fn case_insensitive_pairs_match_qjs() {
    let data = include_str!("../../../../tests/data/regexp/case.txt");
    let mut n = 0;
    let utf16 = |c: u32| {
        let mut b = [0u16; 2];
        char::from_u32(c).unwrap().encode_utf16(&mut b).to_vec()
    };
    for line in data.lines().filter(|l| l.starts_with("eq\t")) {
        let f: Vec<&str> = line.split('\t').collect();
        let c = u32::from_str_radix(f[1], 16).unwrap();
        let x = u32::from_str_radix(f[2], 16).unwrap();
        for (flags, expect) in [("i", f[3]), ("ui", f[4])] {
            if expect == "-" {
                continue;
            }
            let mut pat = u16s("^");
            pat.extend(utf16(c));
            pat.extend(u16s("$"));
            let prog = compile(&pat, Flags::parse(&u16s(flags)).unwrap()).unwrap();
            let input = utf16(x);
            let mut fuel = u64::MAX;
            let got = matches!(
                exec(
                    &prog,
                    Input::Utf16(&input),
                    0,
                    &mut fuel,
                    ExecLimits::default()
                ),
                ExecResult::Match(_)
            );
            assert_eq!(got, expect == "1", "/{c:x}/{flags} on {x:x}");
            // The same pair through a class.
            let mut pat = u16s("^[");
            pat.extend(utf16(c));
            pat.extend(u16s("]$"));
            let prog = compile(&pat, Flags::parse(&u16s(flags)).unwrap()).unwrap();
            let got = matches!(
                exec(
                    &prog,
                    Input::Utf16(&input),
                    0,
                    &mut fuel,
                    ExecLimits::default()
                ),
                ExecResult::Match(_)
            );
            assert_eq!(got, expect == "1", "/[{c:x}]/{flags} on {x:x}");
            n += 1;
        }
    }
    assert!(n > 4000, "{n}");
}

/// Runs with `chunk` fuel at a time, serializing the state between runs.
fn exec_chunked(prog: &Program, input: Input<'_>, start: usize, chunk: u64) -> (ExecResult, usize) {
    let mut fuel = chunk;
    let mut r = exec(prog, input, start, &mut fuel, ExecLimits::default());
    let mut suspensions = 0;
    loop {
        match r {
            ExecResult::OutOfFuel(state) => {
                suspensions += 1;
                let words = state.to_words();
                let state = ExecState::from_words(&words).unwrap();
                assert_eq!(state.to_words(), words);
                fuel = chunk;
                r = resume(prog, input, state, &mut fuel, ExecLimits::default());
            }
            other => return (other, suspensions),
        }
    }
}

fn same_result(a: &ExecResult, b: &ExecResult) -> bool {
    match (a, b) {
        (ExecResult::Match(x), ExecResult::Match(y)) => x == y,
        (ExecResult::NoMatch, ExecResult::NoMatch) => true,
        (ExecResult::StackOverflow, ExecResult::StackOverflow) => true,
        _ => false,
    }
}

#[test]
fn pathological_patterns_run_out_of_fuel() {
    let input = u16s(&"a".repeat(40));
    for p in [
        "(a*)*b",
        "(a+)+b",
        "(a|aa)+b",
        "(a|a)*b",
        "(?:a*)*b",
        "(a*)*(a*)*b",
        "^(a+)+$x",
    ] {
        let prog = re(p, "");
        let mut fuel = 1_000_000;
        let r = exec(
            &prog,
            Input::Utf16(&input),
            0,
            &mut fuel,
            ExecLimits::default(),
        );
        assert!(matches!(r, ExecResult::OutOfFuel(_)), "{p}");
        assert_eq!(fuel, 0);
    }
}

#[test]
fn suspend_and_resume_give_the_same_result() {
    let cases = [
        ("(a*)*b", "", "aaaaaaaaaaab"),
        ("(a*)*b", "", "aaaaaaaaaaa"),
        ("(x+x+)+y", "", "xxxxxxxxxxy"),
        ("^(?:(a)|b)*?c\\1$", "", "babbac"),
        ("(?<=(\\d+)(\\d+))$", "", "12345"),
        ("(?=(a+))a*b\\1", "", "baaabac"),
        ("(.*?)a(?!(a+)b\\2c)\\2(.*)", "", "baaabaac"),
        ("[\\q{abc|ab|a}]+c", "v", "abcababc"),
        ("(?:(\\w)(?:\\1)?)+$", "i", "aAbBcC"),
        ("\\b\\w+(?=,)", "g", "one, two, three"),
        (".+", "su", "😀\n😀"),
    ];
    for (p, f, i) in cases {
        let prog = re(p, f);
        let input = u16s(i);
        let mut fuel = u64::MAX;
        let once = exec(
            &prog,
            Input::Utf16(&input),
            0,
            &mut fuel,
            ExecLimits::default(),
        );
        for chunk in [1, 2, 3, 7, 100] {
            let (r, n) = exec_chunked(&prog, Input::Utf16(&input), 0, chunk);
            assert!(same_result(&once, &r), "/{p}/{f} chunk {chunk}");
            if chunk == 1 {
                assert!(n > 5, "/{p}/{f} suspended only {n} times");
            }
        }
    }
}

#[test]
fn stack_limit() {
    let prog = re("(?:a|b)*c", "");
    let input = u16s(&"a".repeat(10_000));
    let mut fuel = u64::MAX;
    let limits = ExecLimits { max_stack: 1000 };
    let r = exec(&prog, Input::Utf16(&input), 0, &mut fuel, limits);
    assert!(matches!(r, ExecResult::StackOverflow));
    // A plain loop without choice points does not grow the stack.
    let prog = re("a{10000}", "");
    let r = exec(&prog, Input::Utf16(&input), 0, &mut fuel, limits);
    assert!(matches!(r, ExecResult::Match(_)));
    let prog = re("(a){10000}", "");
    let r = exec(&prog, Input::Utf16(&input), 0, &mut fuel, limits);
    assert!(matches!(r, ExecResult::Match(_)));
}

#[test]
fn latin1_input() {
    let prog = re("caf\u{e9}", "i");
    let input = b"un CAF\xc9";
    let mut fuel = u64::MAX;
    match exec(
        &prog,
        Input::Latin1(input),
        0,
        &mut fuel,
        ExecLimits::default(),
    ) {
        ExecResult::Match(c) => assert_eq!(c.get(0), Some((3, 7))),
        _ => panic!(),
    }
}

/// Rough timings. Run with
/// `cargo test -p djs-core --release bench_regexp -- --ignored --nocapture`.
#[test]
#[ignore]
fn bench_regexp() {
    use std::println;
    use std::time::Instant;
    let patterns = [
        ("\\d+", ""),
        ("[\\w.+-]+@[\\w-]+\\.[\\w.]+", ""),
        ("https?://([^/\\s]+)(/[^\\s?#]*)?", ""),
        ("(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", ""),
        ("\\bfox\\b", "i"),
        ("^\\s+|\\s+$", "g"),
        ("[\\p{ASCII}]", "v"),
    ];
    for (p, f) in patterns {
        let pat = u16s(p);
        let flags = Flags::parse(&u16s(f)).unwrap();
        if compile(&pat, flags).is_err() {
            continue;
        }
        let n = 20_000;
        let t = Instant::now();
        for _ in 0..n {
            core::hint::black_box(compile(&pat, flags).unwrap());
        }
        println!(
            "compile /{p}/{f:4} {:8.0} ns",
            t.elapsed().as_nanos() as f64 / n as f64
        );
    }
    let text: String = "The quick brown fox jumps over the lazy dog. Contact john.doe@example.com or visit https://example.com/a/b on 2024-01-15. "
        .repeat(1000);
    let input = u16s(&text);
    let latin1: Vec<u8> = text.bytes().collect();
    for (p, f) in [
        ("[\\w.+-]+@[\\w-]+\\.[\\w.]+", "g"),
        ("\\d+", "g"),
        ("(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", "g"),
        ("\\bfox\\b", "gi"),
        ("lazy dog", "g"),
        ("zebra", "g"),
        ("\\s+", "g"),
    ] {
        let prog = re(p, f);
        for (name, inp) in [
            ("utf16", Input::Utf16(&input)),
            ("latin1", Input::Latin1(&latin1)),
        ] {
            let t = Instant::now();
            let mut start = 0;
            let mut count = 0;
            let mut steps = 0u64;
            loop {
                let mut fuel = u64::MAX;
                match exec(&prog, inp, start, &mut fuel, ExecLimits::default()) {
                    ExecResult::Match(c) => {
                        let (s, e) = c.get(0).unwrap();
                        start = if e == s { e + 1 } else { e };
                        count += 1;
                    }
                    _ => {
                        steps += u64::MAX - fuel;
                        break;
                    }
                }
                steps += u64::MAX - fuel;
            }
            let el = t.elapsed();
            println!(
                "scan /{p}/{f} {name}: {count} matches in {:.2} ms ({:.1} MB/s, {:.1} ns/step)",
                el.as_secs_f64() * 1e3,
                text.len() as f64 / el.as_secs_f64() / 1e6,
                el.as_nanos() as f64 / steps as f64
            );
        }
    }
}
