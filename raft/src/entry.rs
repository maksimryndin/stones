use crate::{LogId, Term};
use std::cmp::{Ordering, PartialEq};

#[derive(PartialEq, Eq)]
pub(crate) struct EntryMeta {
    pub(crate) index: LogId,
    pub(crate) term: Term,
}

/// Raft determines which of two logs is more up-to-date
/// by comparing the index and term of the last entries in the
/// logs. If the logs have last entries with different terms, then
/// the log with the later term is more up-to-date. If the logs
/// end with the same term, then whichever log is longer is
/// more up-to-date
impl Ord for EntryMeta {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.term != other.term {
            self.term.cmp(&other.term)
        } else {
            self.index.cmp(&other.index)
        }
    }
}

impl PartialOrd for EntryMeta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

pub(crate) struct Entry<S> {
    pub(crate) meta: EntryMeta,
    pub(crate) command: S,
}

/// Log Matching: if two logs contain an entry with the same
/// index and term, then the logs are identical in all entries
/// up through the given index.
impl<S> PartialEq for Entry<S> {
    fn eq(&self, other: &Self) -> bool {
        self.meta.eq(&other.meta)
    }
}

impl<S> Eq for Entry<S> {}

impl<S> PartialEq<EntryMeta> for Entry<S> {
    fn eq(&self, other: &EntryMeta) -> bool {
        self.meta.eq(other)
    }
}

impl<S> PartialEq<Entry<S>> for EntryMeta {
    fn eq(&self, other: &Entry<S>) -> bool {
        self.eq(&other.meta)
    }
}

impl<S> Ord for Entry<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.meta.cmp(&other.meta)
    }
}

impl<S> PartialOrd for Entry<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl<S> PartialOrd<EntryMeta> for Entry<S> {
    fn partial_cmp(&self, other: &EntryMeta) -> Option<Ordering> {
        self.meta.partial_cmp(other)
    }
}

impl<S> PartialOrd<Entry<S>> for EntryMeta {
    fn partial_cmp(&self, other: &Entry<S>) -> Option<Ordering> {
        self.partial_cmp(&other.meta)
    }
}
