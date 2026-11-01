//! Minimal glob matching for test paths and feature names.
//!
//! `*` matches any run of characters except `/`, `**` matches any run including `/`
//! (so `a/**/b` and `a/**` work as usual), `?` matches one character except `/`.

pub fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

pub fn matches(pattern: &str, text: &str) -> bool {
    matches_bytes(pattern.as_bytes(), text.as_bytes())
}

fn matches_bytes(p: &[u8], t: &[u8]) -> bool {
    match p.first() {
        None => t.is_empty(),
        Some(b'*') if p.get(1) == Some(&b'*') => {
            let rest = &p[2..];
            // `**/` also matches zero directories.
            if rest.first() == Some(&b'/') && matches_bytes(&rest[1..], t) {
                return true;
            }
            (0..=t.len()).any(|i| matches_bytes(rest, &t[i..]))
        }
        Some(b'*') => {
            let rest = &p[1..];
            for i in 0..=t.len() {
                if matches_bytes(rest, &t[i..]) {
                    return true;
                }
                if i < t.len() && t[i] == b'/' {
                    break;
                }
            }
            false
        }
        Some(b'?') => !t.is_empty() && t[0] != b'/' && matches_bytes(&p[1..], &t[1..]),
        Some(&c) => t.first() == Some(&c) && matches_bytes(&p[1..], &t[1..]),
    }
}

/// True if `path` equals `prefix` or lies below it (`prefix` names a directory).
pub fn has_path_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    prefix.is_empty()
        || path == prefix
        || (path.len() > prefix.len()
            && path.starts_with(prefix)
            && path.as_bytes()[prefix.len()] == b'/')
}

/// A path filter as given on the command line: a glob, or else a path prefix.
pub fn path_filter_matches(filter: &str, path: &str) -> bool {
    if is_glob(filter) {
        matches(filter, path)
    } else {
        has_path_prefix(path, filter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star() {
        assert!(matches("Intl-*", "Intl-enumeration"));
        assert!(matches("Intl.*", "Intl.Locale"));
        assert!(!matches("Intl.*", "Intl-enumeration"));
        assert!(matches("*", "abc"));
        assert!(matches("a*c", "abbbc"));
        assert!(!matches("a/*", "a/b/c"));
        assert!(matches("a/*/c", "a/b/c"));
        assert!(matches("a?c", "abc"));
        assert!(!matches("a?c", "a/c"));
    }

    #[test]
    fn double_star() {
        assert!(matches("a/**", "a/b/c.js"));
        assert!(matches("a/**/c.js", "a/c.js"));
        assert!(matches("a/**/c.js", "a/b/x/c.js"));
        assert!(matches("**/S15*.js", "built-ins/x/S15.1.js"));
        assert!(!matches("a/**/c.js", "b/c.js"));
    }

    #[test]
    fn prefix() {
        assert!(has_path_prefix(
            "built-ins/Temporal/x.js",
            "built-ins/Temporal"
        ));
        assert!(has_path_prefix(
            "built-ins/Temporal/x.js",
            "built-ins/Temporal/"
        ));
        assert!(!has_path_prefix(
            "built-ins/TemporalX/x.js",
            "built-ins/Temporal"
        ));
        assert!(has_path_prefix("a/b.js", "a/b.js"));
        assert!(path_filter_matches(
            "built-ins/Array",
            "built-ins/Array/from/x.js"
        ));
        assert!(path_filter_matches(
            "built-ins/*/from/*.js",
            "built-ins/Array/from/x.js"
        ));
    }
}
