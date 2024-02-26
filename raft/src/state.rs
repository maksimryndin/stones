use core::ops::{Deref, DerefMut};
use std::collections::HashMap;

use crate::entry::Entry;
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

// impl also AsRef an AsMut for Persistent

pub(crate) struct CommonAttributes<S> {
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
}

pub(crate) struct RaftNode<R: Role, S> {
    pub(crate) common: CommonAttributes<S>,
    /// Role-specific data
    pub(crate) role: R,
}

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

    pub(crate) async fn on_append_reponse(
        self,
        response: AppendEntriesResponse,
    ) -> Transition<Leader, Follower, S> {
        if response.term > *self.common.current_term {
            return Transition::ChangedTo(self.into());
        }
        if !response.success {}
        // if last log index ≥ nextIndex for a follower: send
        // AppendEntries RPC with log entries starting at nextIndex
        // If successful: update nextIndex and matchIndex for
        // follower (§5.3)
        // • If AppendEntries fails because of log inconsistency:
        // decrement nextIndex and retry

        // can become a follower it term > current
        // count responses
        // if request was replicated on the majority of nodes
        // 1) respond to client 2) mark entry as commited so all previous entries are also considered commited 3) apply entry to its state machine
        // Only log entries from the leader’s current
        // term are committed by counting replicas; once an entry
        // from the current term has been committed in this way,
        // then all prior entries are committed indirectly because
        // of the Log Matching Property.

        //         If there exists an N such that N > commitIndex, a majority
        // of matchIndex[i] ≥ N, and log[N].term == currentTerm:
        // set commitIndex = N (§5.3, §5.4).

        // if request was rejected, the leader decrements nextIndex for nodeId and retries the AppendEntries RPC
        Transition::Remains(self)
    }

    pub(crate) async fn send_append_requests(&mut self) {
        todo!()
    }
}

pub(crate) struct Candidate {
    votes: usize,
}

impl<S> RaftNode<Candidate, S> {
    pub(crate) async fn on_append_request(
        mut self,
        request: AppendEntriesRequest<S>,
    ) -> Transition<Candidate, Follower, S> {
        let response = self.process_append_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
        if response.success {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    pub(crate) async fn on_vote_request(
        mut self,
        request: RequestVoteRequest,
    ) -> Transition<Candidate, Follower, S> {
        let response = self.process_vote_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
        if response.vote_granted {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    pub(crate) async fn on_vote_response(
        self,
        response: RequestVoteResponse,
    ) -> Transition<Candidate, Follower, S> {
        if response.vote_granted {
            self.role.votes.checked_add(1).expect("too many votes");
        } else if response.term > *self.common.current_term {
            return Transition::ChangedTo(self.into());
        }
        Transition::Remains(self)
    }

    // if majority of votes becomes a new Leader and sends heartbeats
    pub(crate) fn check_votes(self) -> Transition<Candidate, Leader, S> {
        // TODO majority
        if self.role.votes > 3 {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    // start a new election by incrementing its term and initiating another round of Request-Vote RPCs
    pub(crate) async fn on_election_timeout(&mut self) {
        if let Err(_) = (*self.common.current_term).increment() {
            // TODO crash as we cannot insrease term ? or remain a follower?
            // term is persistent so we cannot recover from crash automatically
            // neither receive append requests - every time any leader will be disqualified
        }
        // vote for self
        self.role.votes.checked_add(1).expect("too many votes");
        // broadcast RequestVote
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
        let response = self.process_append_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
    }

    pub(crate) async fn on_vote_request(&mut self, request: RequestVoteRequest) {
        let response = self.process_vote_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
    }

    pub(crate) async fn on_election_timeout(self) -> RaftNode<Candidate, S> {
        let mut candidate: RaftNode<Candidate, S> = self.into();
        candidate.on_election_timeout().await;
        candidate
    }

    // redirect to the leader
    pub(crate) async fn on_client_request(&self, request: ()) {
        todo!()
    }
}

/// State transitions

impl<S> From<RaftNode<Candidate, S>> for RaftNode<Follower, S> {
    fn from(candidate: RaftNode<Candidate, S>) -> Self {
        // initializes all next_index values to the index just after the
        // last one in its log
        todo!()
    }
}

impl<S> From<RaftNode<Leader, S>> for RaftNode<Follower, S> {
    fn from(leader: RaftNode<Leader, S>) -> Self {
        // initializes all next_index values to the index just after the
        // last one in its log
        todo!()
    }
}

impl<S> From<RaftNode<Follower, S>> for RaftNode<Candidate, S> {
    fn from(follower: RaftNode<Follower, S>) -> Self {
        // initializes all next_index values to the index just after the
        // last one in its log
        todo!()
    }
}

impl<S> From<RaftNode<Candidate, S>> for RaftNode<Leader, S> {
    fn from(candidate: RaftNode<Candidate, S>) -> Self {
        // initializes all next_index values to the index just after the
        // last one in its log
        todo!()
    }
}
