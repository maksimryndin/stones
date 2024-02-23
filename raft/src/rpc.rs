use crate::state::{Entry, RaftNode, Role};
use crate::{LogId, NodeId, Term};

/// Invoked by leader to replicate log entries (§5.3); also used as
/// heartbeat (§5.2).
pub(crate) struct AppendEntriesRequest<S> {
    /// leader’s term
    term: Term,
    /// so follower can redirect clients
    leader_id: NodeId,
    /// index of log entry immediately preceding
    /// new ones
    prev_log_index: LogId,
    /// term of prevLogIndex entry
    prev_log_term: Term,
    /// log entries to store (empty for heartbeat;
    /// may send more than one for efficiency)
    entries: Vec<Entry<S>>,
    /// leader’s commitIndex
    leader_commit: LogId,
}

pub(crate) struct AppendEntriesResponse {
    /// currentTerm, for leader to update itself
    term: Term,
    /// true if follower contained entry matching
    /// prevLogIndex and prevLogTerm
    success: bool,
}

trait NodeRequest {
    fn term(&self) -> Term;
}

impl<S> NodeRequest for AppendEntriesRequest<S> {
    fn term(&self) -> Term {
        self.term
    }
}

impl<R: Role, S> RaftNode<R, S> {
    /// Current terms are exchanged
    /// whenever servers communicate
    fn on_node_request(&mut self, req: &dyn NodeRequest) -> bool {
        let current_term = *self.current_term;
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
            *self.current_term = proposed_term;
        }
        true
    }
}

impl<R: Role, S> RaftNode<R, S> {
    // if request is accepted
    // Candidate -> Follower
    // Follower -> Follower
    // If request is not accepted
    // Candidate -> Candidate ?
    // Follower -> Follower
    fn _on_append_request(&mut self, req: AppendEntriesRequest<S>) -> AppendEntriesResponse {
        let current_term = *self.current_term;
        let proposed_term = req.term;
        if !self.on_node_request(&req)
            || self.log.get(req.prev_log_index).map(|entry| entry.term) != Some(req.prev_log_term)
        {
            return AppendEntriesResponse {
                term: current_term,
                success: false,
            };
        }
        let last_index = req.prev_log_index + req.entries.len();

        let (index_delete_since, index_insert_since) = ((req.prev_log_index + 1)..self.log.len())
            .zip(0..req.entries.len())
            .find(|(log_index, new_index)| {
                req.entries[*new_index].term != self.log[*log_index].term
            })
            .unwrap_or((self.log.len(), 0));
        self.log.truncate(index_delete_since);

        for entry in req.entries.into_iter().skip(index_insert_since) {
            self.log.push(entry);
        }

        if req.leader_commit > self.commit_index {
            self.commit_index = req.leader_commit.min(last_index);
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
    /// index of candidate’s last log entry (§5.4)
    last_log_entry: LogId,
    /// term of candidate’s last log entry (§5.4)
    last_log_term: Term,
}

impl NodeRequest for RequestVoteRequest {
    fn term(&self) -> Term {
        self.term
    }
}

pub(crate) struct RequestVoteResponse {
    /// currentTerm, for candidate to update itself
    term: Term,
    /// true means candidate received vote
    vote_granted: bool,
}

impl<R: Role, S> RaftNode<R, S> {
    fn _on_vote_request(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        let current_term = *self.current_term;
        if !self.on_node_request(&req) {
            return RequestVoteResponse {
                term: current_term,
                vote_granted: false,
            };
        }
        //if self.voted_for.is_none() || req.
        // Raft determines which of two logs is more up-to-date
        // by comparing the index and term of the last entries in the
        // logs. If the logs have last entries with different terms, then
        // the log with the later term is more up-to-date. If the logs
        // end with the same term, then whichever log is longer is
        // more up-to-date
        todo!()
    }
}
