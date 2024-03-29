use crate::entry::{Entry, EntryMeta};
use crate::state::{RaftNode, Role};
use crate::{LogId, Term};
use stones_core::NodeId;

/// Invoked by leader to replicate log entries (§5.3); also used as
/// heartbeat (§5.2).
#[derive(Clone)]
pub struct AppendEntriesRequest<C> {
    /// leader’s term
    pub(crate) term: Term,
    /// so follower can redirect clients
    pub(crate) leader_id: NodeId,
    /// a log entry immediately preceding
    /// new ones
    pub(crate) prev_log_entry: Option<EntryMeta>,
    /// log entries to store (empty for heartbeat;
    /// may send more than one for efficiency)
    pub(crate) entries: Vec<Entry<C>>,
    /// leader’s commitIndex
    pub(crate) leader_commit: LogId,
}

pub struct AppendEntriesResponse {
    /// currentTerm, for leader to update itself
    pub(crate) term: Term,
    /// true if follower contained entry matching
    /// prevLogIndex and prevLogTerm
    pub(crate) success: bool,
}

trait NodeRequest {
    fn term(&self) -> Term;
}

impl<C> NodeRequest for AppendEntriesRequest<C> {
    fn term(&self) -> Term {
        self.term
    }
}

impl<R: Role, C: Clone> RaftNode<R, C> {
    /// Current terms are exchanged
    /// whenever servers communicate
    fn is_behind(&mut self, req: &dyn NodeRequest) -> bool {
        let current_term = self.common.persistent.current_term;
        let proposed_term = req.term();
        // If a server receives a request with a stale term
        // number, it rejects the request
        if proposed_term < current_term {
            return false;
        }
        // If a candidate or leader discovers
        // that its term is out of date, it immediately reverts to fol-
        // lower state
        if proposed_term > current_term {
            self.common.persistent.update().current_term = proposed_term;
            self.common.persistent.update().voted_for = None;
        }
        true
    }
}

impl<R: Role, C: Clone> RaftNode<R, C> {
    pub(crate) fn process_append_request(
        &mut self,
        req: AppendEntriesRequest<C>,
    ) -> (bool, AppendEntriesResponse) {
        let current_term = self.common.persistent.current_term;
        let prev_log_index = req.prev_log_entry.as_ref().map(|e| e.index).unwrap_or(0);
        let is_behind = self.is_behind(&req);
        if !is_behind
            || self
                .common
                .persistent
                .log
                .get(prev_log_index)
                .map(|e| &e.meta)
                != req.prev_log_entry.as_ref()
        {
            return (
                is_behind,
                AppendEntriesResponse {
                    term: current_term,
                    success: false,
                },
            );
        }
        let last_index = prev_log_index + req.entries.len();

        let (index_delete_since, index_insert_since) = ((prev_log_index + 1)
            ..self.common.persistent.log.len())
            .zip(0..req.entries.len())
            .find(|(log_index, new_index)| {
                req.entries[*new_index] != self.common.persistent.log[*log_index]
            })
            .unwrap_or((self.common.persistent.log.len(), 0));
        self.common
            .persistent
            .update()
            .log
            .truncate(index_delete_since);

        for entry in req.entries.into_iter().skip(index_insert_since) {
            self.common.persistent.update().log.push(entry);
        }

        if req.leader_commit > self.common.commit_index {
            self.common.commit_index = req.leader_commit.min(last_index);
        }

        (
            false,
            AppendEntriesResponse {
                term: current_term,
                success: true,
            },
        )
    }
}

/// Invoked by candidates to gather votes (§5.2).
#[derive(Clone)]
pub struct RequestVoteRequest {
    /// candidate’s term
    pub(crate) term: Term,
    /// candidate requesting vote
    pub(crate) candidate_id: NodeId,
    /// candidate’s last log entry (§5.4)
    pub(crate) last_log_entry: Option<EntryMeta>,
}

impl NodeRequest for RequestVoteRequest {
    fn term(&self) -> Term {
        self.term
    }
}

pub struct RequestVoteResponse {
    /// currentTerm, for candidate to update itself
    pub(crate) term: Term,
    /// true means candidate received vote
    pub(crate) vote_granted: bool,
}

impl<R: Role, C: Clone> RaftNode<R, C> {
    pub(crate) fn process_vote_request(
        &mut self,
        req: RequestVoteRequest,
    ) -> (bool, RequestVoteResponse) {
        let current_term = self.common.persistent.current_term;
        let is_behind = self.is_behind(&req);
        if !is_behind
            || self.common.persistent.voted_for.is_some()
            || req.last_log_entry.as_ref() < self.common.persistent.log.last().map(|e| &e.meta)
        {
            return (
                is_behind,
                RequestVoteResponse {
                    term: current_term,
                    vote_granted: false,
                },
            );
        }
        self.common.persistent.update().voted_for = Some(req.candidate_id);
        (
            false,
            RequestVoteResponse {
                term: current_term,
                vote_granted: true,
            },
        )
    }
}
