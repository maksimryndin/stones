use core::ops::{Deref, DerefMut};

use crate::{LogId, NodeId, Term};

/// Persistent state
pub(crate) struct Persistent<T: Sized>(T);

impl<T: Sized> Deref for Persistent<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Sized> DerefMut for Persistent<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) struct Entry<S> {
    pub(crate) term: Term,
    pub(crate) state: S,
}

// impl also AsRef an AsMut for Persistent

pub(crate) struct RaftNode<S> {
    /// latest term server has seen (initialized to 0
    /// on first boot, increases monotonically)
    pub(crate) current_term: Persistent<Term>,
    /// candidateId that received vote in current
    /// term (or null if none)
    pub(crate) voted_for: Persistent<Option<NodeId>>,
    /// log entries; each entry contains command
    /// for state machine, and term when entry
    /// was received by leader (first index is 1)
    pub(crate) log: Persistent<Vec<Entry<S>>>,
    /// index of highest log entry known to be
    /// committed (initialized to 0, increases
    /// monotonically)
    pub(crate) commit_index: LogId,
    /// index of highest log entry applied to state
    /// machine (initialized to 0, increases
    /// monotonically)
    pub(crate) last_applied: LogId,
    /// Role-specific data
    pub(crate) role: Role,
}

// impl<R: Role, S> RaftNode<R, S> {
//     fn new() ->
// }

pub(crate) enum Role {
    Leader {
        /// for each server, index of the next log entry
        /// to send to that server (initialized to leader
        /// last log index + 1)
        next_index: LogId,
        /// for each server, index of highest log entry
        /// known to be replicated on server
        /// (initialized to 0, increases monotonically)
        match_index: LogId,
    },
    Candidate,
    Follower,
}
