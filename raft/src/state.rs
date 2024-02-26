use core::ops::{Deref, DerefMut};
use std::collections::HashMap;

use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
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

/// Log Matching: if two logs contain an entry with the same
/// index and term, then the logs are identical in all entries
/// up through the given index.
pub(crate) struct Entry<S> {
    pub(crate) index: LogId,
    pub(crate) term: Term,
    pub(crate) command: S,
}

// impl also AsRef an AsMut for Persistent

pub(crate) struct RaftNode<R: Role, S> {
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
    /// monotonically).
    /// Leader Completeness: if a log entry is committed in a
    /// given term, then that entry will be present in the logs
    /// of the leaders for all higher-numbered terms.
    pub(crate) commit_index: LogId,
    /// index of highest log entry applied to state
    /// machine (initialized to 0, increases
    /// monotonically)
    /// State Machine Safety: if a server has applied a log entry
    /// at a given index to its state machine, no other server
    /// will ever apply a different log entry for the same index.
    pub(crate) last_applied: LogId,
    /// Role-specific data
    pub(crate) role: R,
}

// impl<R: Role, S> RaftNode<R, S> {
//     fn new() -> Self<S> {

//     }

//     fn follower_to_candidate()
// }

pub(crate) trait Role {}

impl Role for Leader {}
impl Role for Candidate {}
impl Role for Follower {}

/// Possible transition from role R to role N
pub(crate) enum Transition<R: Role, N: Role, S> {
    Remains(RaftNode<R, S>),
    ChangedTo(RaftNode<N, S>),
}

pub(crate) struct Leader {
    /// for each server, index of the next log entry
    /// to send to that server (initialized to leader
    /// last log index + 1)
    next_index: HashMap<NodeId, LogId>,
    /// for each server, index of highest log entry
    /// known to be replicated on server
    /// (initialized to 0, increases monotonically)
    match_index: HashMap<NodeId, LogId>,
}

impl<S> RaftNode<Leader, S> {
    // client request contains a command to
    // be executed by the replicated state machines
    // todo request contains command
    pub(crate) async fn on_client_request(&mut self, request: ()) {
        // appends the command to its log as a new entry
        // issues AppendEntries RPCs in parallel to each of the other
        // servers to replicate the entry

        // If followers crash or run slowly,
        // or if network packets are lost, the leader retries Append-
        // Entries RPCs indefinitely (even after it has responded to
        // the client) until all followers eventually store all log en-
        // tries.

        // collected replies
        todo!()
    }

    pub(crate) async fn on_append_reponse(&mut self, response: AppendEntriesResponse) {
        // count responses
        // if request was replicated on the majority of nodes
        // 1) respond to client 2) mark entry as commited so all previous entries are also considered commited 3) apply entry to its state machine
        // Only log entries from the leader’s current
        // term are committed by counting replicas; once an entry
        // from the current term has been committed in this way,
        // then all prior entries are committed indirectly because
        // of the Log Matching Property.

        // if request was rejected, the leader decrements nextIndex for nodeId and retries the AppendEntries RPC
    }
}

impl<S> From<RaftNode<Candidate, S>> for RaftNode<Leader, S> {
    fn from(candidate: RaftNode<Candidate, S>) -> Self {
        // initializes all next_index values to the index just after the
        // last one in its log
        todo!()
    }
}

pub(crate) struct Candidate {}

impl<S> RaftNode<Candidate, S> {
    pub(crate) async fn on_append_request(
        self,
        request: AppendEntriesRequest<S>,
    ) -> Transition<Candidate, Follower, S> {
        // if req.term >= self.current_term -> Follower
        todo!()
    }

    pub(crate) async fn on_vote_request(
        self,
        request: RequestVoteRequest,
    ) -> Transition<Candidate, Follower, S> {
        todo!()
    }

    // either receives new votes and remains Candidate or becomes a new Leader (if majority of votes) and sends heartbeats
    pub(crate) async fn on_vote_response(
        self,
        request: RequestVoteResponse,
    ) -> Transition<Candidate, Leader, S> {
        todo!()
    }

    // start a new election by incrementing its term and initiating another round of Request-Vote RPCs
    pub(crate) async fn on_election_timeout(&mut self) {
        todo!()
    }

    // if candidate (i.e. doesn't know the leader) receives a client request - it responds with 503 Service Unavailable
    pub(crate) async fn on_client_request(&self, request: ()) {
        todo!()
    }
}

/// A server remains in follower state as long as
// it receives valid RPCs from a leader or candidate.
pub(crate) struct Follower {}

impl<S> RaftNode<Follower, S> {
    pub(crate) async fn new() -> RaftNode<Follower, S> {
        todo!()
    }

    pub(crate) async fn on_append_request(&mut self, request: AppendEntriesRequest<S>) {
        // Once a follower learns
        // that a log entry is committed, it applies the entry to its
        // local state machine (in log order)
        todo!()
    }

    pub(crate) async fn on_vote_request(&mut self, request: RequestVoteRequest) {
        todo!()
    }

    pub(crate) async fn on_election_timeout(self) -> RaftNode<Candidate, S> {
        // a follower increments its current
        // term and transitions to candidate state
        // votes for
        // itself and issues RequestVote RPCs in parallel to each of
        // the other servers in the cluster
        todo!()
    }

    // redirect to the leader
    pub(crate) async fn on_client_request(&self, request: ()) {
        todo!()
    }
}
