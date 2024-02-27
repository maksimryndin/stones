use core::ops::{Deref, DerefMut};
use std::collections::HashMap;
use stones_core::StateMachine;

use crate::entry::Entry;
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crate::client::Request;
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

pub(crate) struct CommonAttributes<C> {
    /// latest term server has seen (initialized to 0
    /// on first boot, increases monotonically)
    pub(crate) current_term: Persistent<Term>,
    /// candidateId that received vote in current
    /// term (or null if none)
    pub(crate) voted_for: Persistent<Option<NodeId>>,
    /// log entries; each entry contains command
    /// for state machine, and term when entry
    /// was received by leader (first index is 1)
    pub(crate) log: Persistent<Vec<Entry<C>>>,
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
    // state machine
    pub(crate) machine: StateMachine<C>,
    // nodes
    pub(crate) nodes: HashMap<NodeId, LogId>,
}

pub(crate) struct RaftNode<R: Role, C> {
    pub(crate) common: CommonAttributes<C>,
    /// Role-specific data
    pub(crate) role: R,
}

pub(crate) trait Role {}

impl Role for Leader {}
impl Role for Candidate {}
impl Role for Follower {}

/// Possible transition from role R to role N
pub(crate) enum Transition<R: Role, N: Role, C> {
    Remains(RaftNode<R, C>),
    ChangedTo(RaftNode<N, C>),
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

impl<C> RaftNode<Leader, C> {
    // client request contains a command to
    // be executed by the replicated state machines
    // todo request contains command
    pub(crate) async fn on_client_request(&mut self, request: Request<C>) {
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
    ) -> Transition<Leader, Follower, C> {
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
    votes: u8,
}

impl<C> RaftNode<Candidate, C> {
    pub(crate) async fn on_append_request(
        mut self,
        request: AppendEntriesRequest<C>,
    ) -> Transition<Candidate, Follower, C> {
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
    ) -> Transition<Candidate, Follower, C> {
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
    ) -> Transition<Candidate, Follower, C> {
        if response.vote_granted {
            self.role.votes.checked_add(1).expect("too many votes");
        } else if response.term > *self.common.current_term {
            return Transition::ChangedTo(self.into());
        }
        Transition::Remains(self)
    }

    // if majority of votes becomes a new Leader and sends heartbeats
    pub(crate) fn check_votes(self) -> Transition<Candidate, Leader, C> {
        let number_of_nodes: u8 = self
            .common
            .nodes
            .len()
            .try_into()
            .expect("the maximum number of nodes is 255");
        if self.role.votes > number_of_nodes / 2 {
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
        // TODO save persistent state
    }

    // if candidate (i.e. doesn't know the leader) receives a client request - it responds with 503 Service Unavailable
    pub(crate) async fn on_client_request(&self, request: Request<C>) {
        todo!()
    }
}

/// A server remains in follower state as long as
// it receives valid RPCs from a leader or candidate.
pub(crate) struct Follower {
    leader_id: Option<NodeId>,
}

impl<C> RaftNode<Follower, C> {
    pub(crate) async fn new() -> RaftNode<Follower, C> {
        todo!()
    }

    pub(crate) async fn on_append_request(&mut self, request: AppendEntriesRequest<C>) {
        self.role.leader_id = Some(request.leader_id.clone());
        let response = self.process_append_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
    }

    pub(crate) async fn on_vote_request(&mut self, request: RequestVoteRequest) {
        let response = self.process_vote_request(request);
        // TODO send response via provided reply_to
        // TODO save persistent state
    }

    pub(crate) async fn on_election_timeout(self) -> RaftNode<Candidate, C> {
        let mut candidate: RaftNode<Candidate, C> = self.into();
        candidate.on_election_timeout().await;
        candidate
    }

    // redirect to the leader
    pub(crate) async fn on_client_request(&self, request: Request<C>, reply_to: ) {
        todo!()
    }
}

/// State transitions

impl<C> From<RaftNode<Candidate, C>> for RaftNode<Follower, C> {
    fn from(candidate: RaftNode<Candidate, C>) -> Self {
        let RaftNode::<Candidate, C> { common, .. } = candidate;
        RaftNode::<Follower, C> {
            common,
            role: Follower { leader_id: None },
        }
    }
}

impl<C> From<RaftNode<Leader, C>> for RaftNode<Follower, C> {
    fn from(leader: RaftNode<Leader, C>) -> Self {
        let RaftNode::<Leader, C> { common, .. } = leader;
        RaftNode::<Follower, C> {
            common,
            role: Follower { leader_id: None },
        }
    }
}

impl<C> From<RaftNode<Follower, C>> for RaftNode<Candidate, C> {
    fn from(follower: RaftNode<Follower, C>) -> Self {
        let RaftNode::<Follower, C> { common, .. } = follower;
        RaftNode::<Candidate, C> {
            common,
            role: Candidate { votes: 0 },
        }
    }
}

impl<C> From<RaftNode<Candidate, C>> for RaftNode<Leader, C> {
    fn from(candidate: RaftNode<Candidate, C>) -> Self {
        let RaftNode::<Candidate, C> { common, .. } = candidate;
        let log_length = common.log.len();
        let next_index = common
            .nodes
            .keys()
            .cloned()
            .map(|node_id| (node_id, log_length))
            .collect();
        let match_index = common
            .nodes
            .keys()
            .cloned()
            .map(|node_id| (node_id, 0))
            .collect();
        RaftNode::<Leader, C> {
            common,
            role: Leader {
                next_index,
                match_index,
            },
        }
    }
}
