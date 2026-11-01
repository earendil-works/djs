//! Parallel map on std threads.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

/// Maps `f` over `items` on `jobs` threads, returning results in input order.
pub fn map<T, R, F>(items: Vec<T>, jobs: usize, f: F) -> Vec<R>
where
    T: Send + Sync,
    R: Send,
    F: Fn(T) -> R + Sync,
{
    map_with_progress(items, jobs, f, |_, _| {})
}

/// Like `map`, and calls `progress(done_count, &result)` on the calling thread as
/// results arrive (in completion order).
pub fn map_with_progress<T, R, F, P>(items: Vec<T>, jobs: usize, f: F, mut progress: P) -> Vec<R>
where
    T: Send + Sync,
    R: Send,
    F: Fn(T) -> R + Sync,
    P: FnMut(usize, &R),
{
    let n = items.len();
    // Workers claim indexes from `next`; each slot is taken exactly once.
    let slots: Vec<std::sync::Mutex<Option<T>>> = items
        .into_iter()
        .map(|t| std::sync::Mutex::new(Some(t)))
        .collect();
    let next = AtomicUsize::new(0);
    let mut results: Vec<Option<R>> = (0..n).map(|_| None).collect();
    let jobs = jobs.clamp(1, n.max(1));
    thread::scope(|s| {
        let (tx, rx) = mpsc::channel();
        for _ in 0..jobs {
            let tx = tx.clone();
            let (slots, next, f) = (&slots, &next, &f);
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let item = slots[i].lock().unwrap().take().unwrap();
                    if tx.send((i, f(item))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (done, (i, r)) in rx.into_iter().enumerate() {
            progress(done + 1, &r);
            results[i] = Some(r);
        }
    });
    results
        .into_iter()
        .map(|r| r.expect("worker panicked"))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order() {
        let v: Vec<u32> = (0..1000).collect();
        let mut seen = 0;
        let out = super::map_with_progress(v, 8, |x| x * 2, |done, _| seen = done);
        assert_eq!(out, (0..1000).map(|x| x * 2).collect::<Vec<_>>());
        assert_eq!(seen, 1000);
        assert!(super::map(Vec::<u32>::new(), 4, |x| x).is_empty());
    }
}
