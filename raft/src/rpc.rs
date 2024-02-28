use crate::entry::{Entry, EntryMeta};
use crate::state::{RaftNode, Role};
use crate::{LogId, Term};
use stones_core::{NodeId, StateMachine};

/// Invoked by leader to replicate log entries (§5.3); also used as
/// heartbeat (§5.2).
pub(crate) struct AppendEntriesRequest<C> {
    /// leader’s term
    pub(crate) term: Term,
    /// so follower can redirect clients
    pub(crate) leader_id: NodeId,
    /// a log entry immediately preceding
    /// new ones
    pub(crate) prev_log_entry: EntryMeta,
    /// log entries to store (empty for heartbeat;
    /// may send more than one for efficiency)
    pub(crate) entries: Vec<Entry<C>>,
    /// leader’s commitIndex
    pub(crate) leader_commit: LogId,
}

pub(crate) struct AppendEntriesResponse {
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

impl<R: Role, C, S: StateMachine<C>> RaftNode<R, C, S> {
    /// Current terms are exchanged
    /// whenever servers communicate
    fn on_node_request(&mut self, req: &dyn NodeRequest) -> bool {
        let current_term = *self.common.current_term;
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
            *self.common.current_term = proposed_term;
        }
        true
    }
}

impl<R: Role, C, S: StateMachine<C>> RaftNode<R, C, S> {
    pub(crate) fn process_append_request(
        &mut self,
        req: AppendEntriesRequest<C>,
    ) -> AppendEntriesResponse {
        let current_term = *self.common.current_term;
        let prev_log_index = req.prev_log_entry.index;
        if !self.on_node_request(&req)
            || prev_log_index >= self.common.log.len()
            || &self.common.log[prev_log_index] != &req.prev_log_entry
        {
            return AppendEntriesResponse {
                term: current_term,
                success: false,
            };
        }
        let last_index = prev_log_index + req.entries.len();

        let (index_delete_since, index_insert_since) = ((prev_log_index + 1)
            ..self.common.log.len())
            .zip(0..req.entries.len())
            .find(|(log_index, new_index)| req.entries[*new_index] != self.common.log[*log_index])
            .unwrap_or((self.common.log.len(), 0));
        self.common.log.truncate(index_delete_since);

        for entry in req.entries.into_iter().skip(index_insert_since) {
            self.common.log.push(entry);
        }

        if req.leader_commit > self.common.commit_index {
            self.common.commit_index = req.leader_commit.min(last_index);
        }

        AppendEntriesResponse {
            term: current_term,
            success: true,
        }
    }
}

/// Invoked by candidates to gather votes (§5.2).
pub(crate) struct RequestVoteRequest {
    /// candidate’s term
    term: Term,
    /// candidate requesting vote
    candidate_id: NodeId,
    /// candidate’s last log entry (§5.4)
    last_log_entry: EntryMeta,
}

impl NodeRequest for RequestVoteRequest {
    fn term(&self) -> Term {
        self.term
    }
}

pub(crate) struct RequestVoteResponse {
    /// currentTerm, for candidate to update itself
    pub(crate) term: Term,
    /// true means candidate received vote
    pub(crate) vote_granted: bool,
}

impl<R: Role, C, S: StateMachine<C>> RaftNode<R, C, S> {
    pub(crate) fn process_vote_request(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        let current_term = *self.common.current_term;
        if !self.on_node_request(&req)
            || self.common.voted_for.is_some()
            || !self.common.log.is_empty()
                && req.last_log_entry < self.common.log[self.common.log.len() - 1]
        {
            return RequestVoteResponse {
                term: current_term,
                vote_granted: false,
            };
        }
        *self.common.voted_for = Some(req.candidate_id);

        RequestVoteResponse {
            term: current_term,
            vote_granted: true,
        }
    }
}
