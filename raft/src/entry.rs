use crate::{LogId, Term};
use std::cmp::{Ordering, PartialEq};
use stones_core::NodeId;

#[derive(Clone, PartialEq, Eq)]
pub struct EntryMeta {
    pub index: LogId,
    pub term: Term,
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

#[derive(Clone)]
pub struct Entry<C> {
    pub meta: EntryMeta,
    pub command: C,
    pub client_id: NodeId,
}

/// Log Matching: if two logs contain an entry with the same
/// index and term, then the logs are identical in all entries
/// up through the given index.
impl<C> PartialEq for Entry<C> {
    fn eq(&self, other: &Self) -> bool {
        self.meta.eq(&other.meta)
    }
}

impl<C> Eq for Entry<C> {}

impl<C> PartialEq<EntryMeta> for Entry<C> {
    fn eq(&self, other: &EntryMeta) -> bool {
        self.meta.eq(other)
    }
}

impl<C> PartialEq<Entry<C>> for EntryMeta {
    fn eq(&self, other: &Entry<C>) -> bool {
        self.eq(&other.meta)
    }
}

impl<C> Ord for Entry<C> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.meta.cmp(&other.meta)
    }
}

impl<C> PartialOrd for Entry<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl<C> PartialOrd<EntryMeta> for Entry<C> {
    fn partial_cmp(&self, other: &EntryMeta) -> Option<Ordering> {
        self.meta.partial_cmp(other)
    }
}

impl<C> PartialOrd<Entry<C>> for EntryMeta {
    fn partial_cmp(&self, other: &Entry<C>) -> Option<Ordering> {
        self.partial_cmp(&other.meta)
    }
}
