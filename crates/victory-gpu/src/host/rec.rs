//! A host list's D3D12 list, with barrier calls joined
//! (`VICTORY_JOIN_BARRIERS=1`; off by default; A/B-able with
//! `VICTORY_AB=<s>,VICTORY_JOIN_BARRIERS`).
//!
//! The state changes the host asks for (`crate::list::ensure_state`, the
//! aliasing before a texture's first upload) and the title's own barriers
//! wait here instead of going to the list at once; they go, in the order
//! they were asked for, in one call, at the next use of the list for
//! anything else. Every use goes through [`Rec`]'s `Deref`, so nothing is
//! recorded before a barrier asked for earlier. In a replay of capture
//! replay5 joining the calls that had no work between them saved 2.4 ms of
//! 79.5 (2,490 calls joined, 1,619 of them a texture's state change after
//! its upload followed by the next texture's aliasing).

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use windows::Win32::Graphics::Direct3D12::*;

/// Whether barriers are held now (`VICTORY_JOIN_BARRIERS=1`, or its phase
/// of `VICTORY_AB`).
pub(crate) fn on() -> bool {
    #[cfg(feature = "diagnostics")]
    if let Some(on) = crate::ab::pick("VICTORY_JOIN_BARRIERS") {
        return on;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_JOIN_BARRIERS").is_some_and(|v| v == "1"))
}

/// Barriers held. Their resources are borrowed: the list's states hold the
/// textures (`HostList::transition`), and the title's barriers name what
/// the title keeps alive until the list has run.
struct Held {
    barriers: Vec<D3D12_RESOURCE_BARRIER>,
    keys: BarrierKeys,
}

/// Index only the resource identities that the original `named` check
/// considered. A small batch needs no heap allocation for its keys; a long
/// batch avoids repeated scans. Keys never change the recording order.
struct BarrierKeys {
    inline: [usize; 32],
    used: usize,
    indexed: bool,
    large: HashSet<usize>,
}

impl BarrierKeys {
    fn new() -> Self {
        Self { inline: [0; 32], used: 0, indexed: false, large: HashSet::new() }
    }

    fn contains(&self, key: usize) -> bool {
        if self.indexed {
            self.large.contains(&key)
        } else {
            self.inline[..self.used].contains(&key)
        }
    }

    fn add(&mut self, key: usize) {
        if self.indexed {
            self.large.insert(key);
        } else if !self.inline[..self.used].contains(&key) {
            if self.used < self.inline.len() {
                self.inline[self.used] = key;
                self.used += 1;
            } else {
                self.large.extend(self.inline);
                self.large.insert(key);
                self.indexed = true;
            }
        }
    }

    fn clear(&mut self) {
        self.used = 0;
        self.indexed = false;
        self.large.clear();
    }
}

impl Held {
    fn clear(&mut self) {
        self.barriers.clear();
        self.keys.clear();
    }
}

// SAFETY: borrowed interface pointers of free-threaded objects.
unsafe impl Send for Held {}

/// Since the last frame-rate line: barrier calls asked for while held, and
/// calls made of them.
pub(crate) static STATS: [std::sync::atomic::AtomicU64; 2] = [const { std::sync::atomic::AtomicU64::new(0) }; 2];

pub(crate) struct Rec {
    list: ID3D12GraphicsCommandList,
    any: AtomicBool,
    held: Mutex<Held>,
}

impl Rec {
    pub(crate) fn new(list: ID3D12GraphicsCommandList) -> Self {
        Self { list, any: AtomicBool::new(false), held: Mutex::new(Held { barriers: Vec::new(), keys: BarrierKeys::new() }) }
    }

    /// Records `barriers` now, or holds them for the next use of the list.
    ///
    /// # Safety
    /// As `ResourceBarrier`'s: an open list, live resources.
    pub(crate) unsafe fn barriers(&self, barriers: &[D3D12_RESOURCE_BARRIER]) {
        if barriers.is_empty() {
            return;
        }
        if !on() {
            // (What was held while it was on goes first.)
            if self.any.load(Relaxed) {
                self.flush();
            }
            // SAFETY: the caller's.
            unsafe { self.list.ResourceBarrier(barriers) };
            return;
        }
        let mut held = self.held.lock().unwrap_or_else(|e| e.into_inner());
        // A resource whose state changes again: what is held goes first, so
        // that no call holds two changes of one resource.
        if barriers.iter().filter_map(named).any(|r| held.keys.contains(r)) {
            // SAFETY: as in `flush`.
            unsafe { self.list.ResourceBarrier(&held.barriers) };
            held.clear();
            STATS[1].fetch_add(1, Relaxed);
        }
        // SAFETY: a plain copy of the descriptions; nothing is released
        // through them (their pointers are borrowed, `ManuallyDrop`).
        for barrier in barriers {
            if let Some(key) = named(barrier) {
                held.keys.add(key);
            }
            held.barriers.push(unsafe { std::ptr::read(barrier) });
        }
        STATS[0].fetch_add(1, Relaxed);
        self.any.store(true, Relaxed);
    }

    /// What is held goes to the list, in one call.
    fn flush(&self) {
        let mut held = self.held.lock().unwrap_or_else(|e| e.into_inner());
        self.any.store(false, Relaxed);
        if held.barriers.is_empty() {
            return;
        }
        // SAFETY: the barriers were asked for on this open list, in order;
        // their resources are alive (`Held`).
        unsafe { self.list.ResourceBarrier(&held.barriers) };
        held.clear();
        STATS[1].fetch_add(1, Relaxed);
    }
}

/// The resource a state change or aliasing names (its interface pointer).
fn named(b: &D3D12_RESOURCE_BARRIER) -> Option<usize> {
    // SAFETY: the union's member is the one `Type` says; a pointer read.
    unsafe {
        let p: Option<&ID3D12Resource> = match b.Type {
            D3D12_RESOURCE_BARRIER_TYPE_TRANSITION => b.Anonymous.Transition.pResource.as_ref(),
            D3D12_RESOURCE_BARRIER_TYPE_ALIASING => b.Anonymous.Aliasing.pResourceAfter.as_ref(),
            _ => None,
        };
        p.map(|r| windows::core::Interface::as_raw(r) as usize)
    }
}

impl std::ops::Deref for Rec {
    type Target = ID3D12GraphicsCommandList;

    /// The list, with what is held recorded first.
    fn deref(&self) -> &ID3D12GraphicsCommandList {
        if self.any.load(Relaxed) {
            self.flush();
        }
        &self.list
    }
}

/// For the frame-rate line.
pub(crate) fn line(frames: f64) {
    let [asked, made] = [0, 1].map(|i| STATS[i].swap(0, Relaxed) as f64 / frames);
    if asked > 0.0 {
        tracing::info!(
            "barrier calls per frame (VICTORY_JOIN_BARRIERS): {asked:.0} asked for, {made:.0} made of them"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::BarrierKeys;

    /// Reference batching: unnamed barriers never conflict. An incoming call
    /// is compared only with the prior batch, not with itself.
    fn reference(calls: &[Vec<Option<usize>>]) -> Vec<usize> {
        let mut held = Vec::new();
        let mut flushes = Vec::new();
        for (i, call) in calls.iter().enumerate() {
            if call.iter().flatten().any(|key| held.iter().flatten().any(|h| h == key)) {
                flushes.push(i);
                held.clear();
            }
            held.extend_from_slice(call);
        }
        flushes
    }

    fn indexed(calls: &[Vec<Option<usize>>]) -> Vec<usize> {
        let mut keys = BarrierKeys::new();
        let mut flushes = Vec::new();
        for (i, call) in calls.iter().enumerate() {
            if call.iter().flatten().any(|&key| keys.contains(key)) {
                flushes.push(i);
                keys.clear();
            }
            for &key in call.iter().flatten() {
                keys.add(key);
            }
        }
        flushes
    }

    #[test]
    fn barrier_index_preserves_flush_boundaries() {
        let mut seed = 0x4c13_a80du32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as usize
        };
        for domain in [1, 8, 32, 33, 128, 1024, 8192] {
            for _ in 0..32 {
                let calls: Vec<Vec<_>> = (0..512).map(|_| {
                    (0..(next() % 5)).map(|_| {
                        let n = next();
                        (n % 7 != 0).then_some(n % domain)
                    }).collect()
                }).collect();
                assert_eq!(indexed(&calls), reference(&calls));
            }
        }
        // No intra-call split; promotion, duplicate keys, unnamed entries,
        // clearing, and identical IDs reused after a flush.
        let mut calls: Vec<_> = (0..64).map(|i| vec![Some(i), None]).collect();
        calls.extend([vec![Some(31), Some(31)], vec![None], vec![Some(63)], vec![Some(31)]]);
        assert_eq!(indexed(&calls), reference(&calls));
        assert_eq!(indexed(&calls), [64, 67]);
        let mut keys = BarrierKeys::new();
        for i in 0..32 { keys.add(i); }
        assert!(!keys.indexed);
        keys.add(32);
        assert!(keys.indexed);
        for i in 0..33 { assert!(keys.contains(i)); }
        keys.clear();
        assert!(!keys.indexed);
        assert!(!keys.contains(32));
        keys.add(usize::MAX);
        assert!(keys.contains(usize::MAX));
    }

    #[test]
    #[ignore]
    fn benchmark_barrier_hazard_index() {
        use std::hint::black_box;
        use std::time::Instant;
        for count in [16usize, 128, 1024, 2800] {
            for repeated in [false, true] {
                let calls: Vec<_> = (0..count).map(|i| {
                    vec![Some(if repeated { i % 16 } else { i }), None]
                }).collect();
                assert_eq!(reference(&calls), indexed(&calls));
                let measure = |new: bool| {
                    let t = Instant::now();
                    for _ in 0..8 {
                        let out = if new { indexed(black_box(&calls)) } else { reference(black_box(&calls)) };
                        drop(black_box(out));
                    }
                    t.elapsed().as_secs_f64() * 1e6 / 8.0
                };
                black_box(measure(false));
                black_box(measure(true));
                let (mut old, mut new) = (Vec::new(), Vec::new());
                for i in 0..7 {
                    let (a, b) = if i % 2 == 0 {
                        (measure(false), measure(true))
                    } else {
                        let b = measure(true);
                        (measure(false), b)
                    };
                    old.push(a);
                    new.push(b);
                }
                old.sort_by(f64::total_cmp);
                new.sort_by(f64::total_cmp);
                println!("kind=barrier_hazards_{} blocks={count} reference_us={:.3} worklist_us={:.3} speedup={:.3}",
                    if repeated { "repeated" } else { "unique" }, old[3], new[3], old[3] / new[3]);
            }
        }
    }
}
