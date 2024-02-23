use core::ops::{Deref, DerefMut};
use futures::channel::oneshot;
use futures::stream::{self, StreamExt};
use futures::FutureExt;
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
struct Transition<R: Role, N: Role, S> {
    role: Option<RaftNode<R, S>>,
    changed_to: Option<RaftNode<N, S>>,
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
    async fn on_client_request(&mut self, request: ()) {
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

    async fn on_append_reponse(&mut self, response: AppendEntriesResponse) {
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
    async fn on_append_request(
        self,
        request: AppendEntriesRequest<S>,
    ) -> Transition<Candidate, Follower, S> {
        // if req.term >= self.current_term -> Follower
        todo!()
    }

    async fn on_vote_request(
        self,
        request: RequestVoteRequest,
    ) -> Transition<Candidate, Follower, S> {
        todo!()
    }

    // either receives new votes and remains Candidate or becomes a new Leader (if majority of votes) and sends heartbeats
    async fn on_vote_response(
        self,
        request: RequestVoteResponse,
    ) -> Transition<Candidate, Leader, S> {
        todo!()
    }

    // start a new election by incrementing its term and initiating another round of Request-Vote RPCs
    async fn on_election_timeout(&mut self) {
        todo!()
    }

    // if candidate (i.e. doesn't know the leader) receives a client request - it responds with 503 Service Unavailable
    async fn on_client_request(&self, request: ()) {
        todo!()
    }
}

/// A server remains in follower state as long as
// it receives valid RPCs from a leader or candidate.
pub(crate) struct Follower {}

impl<S> RaftNode<Follower, S> {
    async fn new() -> RaftNode<Follower, S> {
        todo!()
    }

    async fn on_append_request(&mut self, request: AppendEntriesRequest<S>) {
        // Once a follower learns
        // that a log entry is committed, it applies the entry to its
        // local state machine (in log order)
        todo!()
    }

    async fn on_vote_request(&mut self, request: RequestVoteRequest) {
        todo!()
    }

    async fn on_election_timeout(self) -> RaftNode<Candidate, S> {
        // a follower increments its current
        // term and transitions to candidate state
        // votes for
        // itself and issues RequestVote RPCs in parallel to each of
        // the other servers in the cluster
        todo!()
    }

    // redirect to the leader
    async fn on_client_request(&self, request: ()) {
        todo!()
    }
}

async fn generate_timeout() -> u64 {
    let (sender, receiver) = oneshot::channel::<u64>();
    let _ = sender.send(3);
    receiver.await.unwrap()
}

pub async fn main<S>() {
    let mut follower_slot = Some(RaftNode::<Follower, S>::new().await);
    let election_timeout = generate_timeout().fuse();
    futures::pin_mut!(election_timeout);
    let mut candidate_slot: Option<RaftNode<Candidate, S>> = None;
    let mut leader_slot: Option<RaftNode<Leader, S>> = None;
    let mut append_requests = stream::iter(vec![]).fuse();
    let mut append_requests_responses = stream::iter(vec![]).fuse();
    let mut vote_requests = stream::iter(vec![]).fuse();
    let mut vote_request_responses = stream::iter(vec![]).fuse();
    let mut client_requests = stream::iter(vec![]).fuse();

    // on any transition to Follower - generate a new election timeout
    loop {
        futures::select_biased! {
            req = &mut append_requests.next() => {
                let req = req.unwrap();
                match (follower_slot.take(), candidate_slot.take()) {
                (Some(mut follower), None) => {
                    follower.on_append_request(req).await;
                },
                (None, Some(candidate)) => {
                    match candidate.on_append_request(req).await {
                        Transition{role: None, changed_to: Some(follower)} => {
                            follower_slot = Some(follower);
                            election_timeout.as_mut().set(generate_timeout().fuse());
                        },
                        _ => unreachable!(),
                    }
                },
                _ => unreachable!()
            }},
            response = &mut append_requests_responses.next() => {
                let response = response.unwrap();
                if let Some(leader) = leader_slot.as_mut() {
                    leader.on_append_reponse(response).await;
            }},
            req = &mut vote_requests.next() => {
                let req = req.unwrap();
                match (follower_slot.take(), candidate_slot.take()) {
                (Some(mut follower), None) => {
                    follower.on_vote_request(req).await;
                },
                (None, Some(candidate)) => {
                    match candidate.on_vote_request(req).await {
                        Transition{role: Some(candidate), changed_to: None} => {candidate_slot = Some(candidate);},
                        Transition{role: None, changed_to: Some(follower)} => {
                            election_timeout.as_mut().set(generate_timeout().fuse());
                            follower_slot = Some(follower);
                        },
                        _ => unreachable!(),
                    }
                },
                _ => unreachable!(),
            }},
            response = &mut vote_request_responses.next() => {
                let response = response.unwrap();
                if let Some(candidate) = candidate_slot.take() {
                match candidate.on_vote_response(response).await {
                    Transition{role: Some(candidate), changed_to: None} => {candidate_slot = Some(candidate);},
                    Transition{role: None, changed_to: Some(leader)} => {leader_slot = Some(leader);},
                    _ => unreachable!(),
                }
            }},
            timeout = &mut election_timeout => match (follower_slot.take(), candidate_slot.as_mut()) {
                (Some(follower), None) => {
                    election_timeout.as_mut().set(generate_timeout().fuse());
                    candidate_slot = Some(follower.on_election_timeout().await);
                },
                (None, Some(candidate)) => {
                    election_timeout.as_mut().set(generate_timeout().fuse());
                    Some(candidate.on_election_timeout().await);
                },
                _ => unreachable!(),
            },
            req = &mut client_requests.next() => {
                let req = req.unwrap();
                match (follower_slot.as_ref(), candidate_slot.as_ref(), leader_slot.as_mut()) {
                (Some(follower), None, None) => follower.on_client_request(req).await,
                (None, Some(candidate), None) => candidate.on_client_request(req).await,
                (None, None, Some(leader)) => leader.on_client_request(req).await,
                _ => unreachable!(),
            }},
            default => todo!("handle  graceful shutdown; nodes reconfiguration"),
        };
    }
}
