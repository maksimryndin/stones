use crate::client::{ClientRequest, ClientResponse};
use crate::effects::Persistence;
use crate::entry::{Entry, EntryMeta};
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crate::{LogId, Term};
use futures::channel::mpsc;
use std::collections::HashMap;
use stones_core::NodeId;

#[derive(Clone)]
pub struct Persistent<C: Clone> {
    /// latest term server has seen (initialized to 0
    /// on first boot, increases monotonically)
    pub current_term: Term,
    /// candidateId that received vote in current
    /// term (or null if none)
    pub voted_for: Option<NodeId>,
    /// log entries; each entry contains command
    /// for state machine, and term when entry
    /// was received by leader (first index is 1)
    pub log: Vec<Entry<C>>,
}

pub(crate) struct CommonAttributes<C: Clone> {
    pub(crate) persistent: Persistence<Persistent<C>>,
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
    pub(crate) machine: mpsc::Sender<C>,
    // nodes
    pub(crate) nodes: Vec<NodeId>,
    // me
    pub(crate) me: NodeId,
}

pub(crate) struct RaftNode<R: Role, C: Clone> {
    pub(crate) common: CommonAttributes<C>,
    /// Role-specific data
    pub(crate) role: R,
}

impl<R: Role, C: Clone> RaftNode<R, C> {
    fn half(&self) -> u8 {
        let number_of_nodes: u8 = self
            .common
            .nodes
            .len()
            .try_into()
            .expect("the maximum number of nodes is 255");
        number_of_nodes / 2
    }
}

pub(crate) trait Role {}

impl Role for Leader {}
impl Role for Candidate {}
impl Role for Follower {}

/// Possible transition from role R to role N
pub(crate) enum Transition<R: Role, N: Role, C: Clone> {
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

impl<C: Clone> RaftNode<Leader, C> {
    // client request contains a command to
    // be executed by the replicated state machines
    pub(crate) fn on_client_request(
        &mut self,
        client_id: NodeId,
        request: ClientRequest<C>,
        multicast_to: &mut mpsc::Sender<Vec<(NodeId, AppendEntriesRequest<C>)>>,
    ) {
        let ClientRequest { command } = request;
        let index = self.common.persistent.log.len();
        let term = self.common.persistent.current_term;
        // appends the command to its log as a new entry
        let new_entry = Entry {
            client_id,
            command,
            meta: EntryMeta { index, term },
        };
        self.common.persistent.update().log.push(new_entry.clone());

        // issues AppendEntries RPCs to other
        // nodes to replicate the entry
        let requests = self
            .common
            .nodes
            .iter()
            .filter_map(|node_id| {
                (node_id != &self.common.me)
                    .then_some((node_id.clone(), self.prepare_append_request(node_id)))
            })
            .collect();
        multicast_to
            .try_send(requests)
            .expect("channel has a free slot");
    }

    pub(crate) fn on_append_reponse(
        mut self,
        node_id: NodeId,
        response: AppendEntriesResponse,
        reply_to_client: &mut mpsc::Sender<(NodeId, ClientResponse)>,
        reply_to: &mut mpsc::Sender<Vec<(NodeId, AppendEntriesRequest<C>)>>,
    ) -> Transition<Leader, Follower, C> {
        if response.term > self.common.persistent.current_term {
            self.common.persistent.update().current_term = response.term;
            return Transition::ChangedTo(self.into());
        }

        if !response.success {
            // If AppendEntries fails because of log inconsistency:
            // decrement nextIndex and retry
            // if last log index ≥ nextIndex for a follower: send
            // AppendEntries RPC with log entries starting at nextIndex
            if let Some(next_index) = self.role.next_index.get_mut(&node_id) {
                *next_index -= 1;
                let req = self.prepare_append_request(&node_id);
                reply_to
                    .try_send(vec![(node_id, req)])
                    .expect("channel has a free slot");
            }
            return Transition::Remains(self);
        }
        // If successful: update nextIndex and matchIndex for
        // follower (§5.3)
        let log_length = self.common.persistent.log.len();
        if log_length > 0 {
            self.role
                .next_index
                .get_mut(&node_id)
                .map(|next_index| *next_index = log_length);
            self.role
                .match_index
                .get_mut(&node_id)
                .map(|match_index| *match_index = log_length - 1);
        }

        // If there exists an N such that N > commitIndex, a majority
        // of matchIndex[i] ≥ N, and log[N].term == currentTerm:
        // set commitIndex = N (§5.3, §5.4).
        // Only log entries from the leader’s current
        // term are committed by counting replicas; once an entry
        // from the current term has been committed in this way,
        // then all prior entries are committed indirectly because
        // of the Log Matching Property.
        let commit_index = self.common.commit_index;
        let current_term = self.common.persistent.current_term;
        // TODO quadratic complexity - redesign match_index data structure??
        for index in self
            .common
            .persistent
            .log
            .iter()
            .rev()
            .filter(|e| e.meta.term == current_term && e.meta.index > commit_index)
            .map(|e| e.meta.index)
        {
            if self
                .role
                .match_index
                .values()
                .filter(|log_index| **log_index >= index)
                .count()
                > self.half().into()
            {
                self.common.commit_index = index;
                break;
            }
        }

        for index in (self.common.last_applied + 1)..(self.common.commit_index + 1) {
            let entry = self.common.persistent.log[index].clone();
            let Entry {
                command, client_id, ..
            } = entry;
            self.common
                .machine
                .try_send(command)
                .expect("channel has a free slot");
            reply_to_client
                .try_send((client_id, ClientResponse::Commited))
                .expect("channel has a free slot");
        }
        self.common.last_applied = self.common.commit_index;
        Transition::Remains(self)
    }

    fn prepare_append_request(&self, node_id: &NodeId) -> AppendEntriesRequest<C> {
        let next_index = *self.role.next_index.get(node_id).expect("node is listed");
        let term = self.common.persistent.current_term;
        let prev_log_entry =
            (next_index != 0).then_some(self.common.persistent.log[next_index - 1].meta.clone());
        AppendEntriesRequest {
            term,
            leader_id: self.common.me.clone(),
            prev_log_entry,
            entries: self.common.persistent.log[next_index..].to_vec(),
            leader_commit: self.common.commit_index,
        }
    }
}

pub(crate) struct Candidate {
    votes: u8,
}

impl<C: Clone> RaftNode<Candidate, C> {
    pub(crate) fn on_append_request(
        mut self,
        node_id: NodeId,
        request: AppendEntriesRequest<C>,
        reply_to: &mut mpsc::Sender<(NodeId, AppendEntriesResponse)>,
    ) -> Transition<Candidate, Follower, C> {
        let (transition, response) = self.process_append_request(request);
        reply_to
            .try_send((node_id, response))
            .expect("channel has a free slot");
        if transition {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    pub(crate) fn on_vote_request(
        mut self,
        node_id: NodeId,
        request: RequestVoteRequest,
        reply_to: &mut mpsc::Sender<(NodeId, RequestVoteResponse)>,
    ) -> Transition<Candidate, Follower, C> {
        let (transition, response) = self.process_vote_request(request);
        reply_to
            .try_send((node_id, response))
            .expect("channel has a free slot");
        if transition {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    pub(crate) fn on_vote_response(
        mut self,
        _node_id: NodeId,
        response: RequestVoteResponse,
    ) -> Transition<Candidate, Follower, C> {
        if response.vote_granted {
            self.role.votes.checked_add(1).expect("too many votes");
        } else if response.term > self.common.persistent.current_term {
            self.common.persistent.update().current_term = response.term;
            return Transition::ChangedTo(self.into());
        }
        Transition::Remains(self)
    }

    // if majority of votes => becomes a new Leader and sends heartbeats
    pub(crate) fn check_votes(self) -> Transition<Candidate, Leader, C> {
        if self.role.votes > self.half() {
            Transition::ChangedTo(self.into())
        } else {
            Transition::Remains(self)
        }
    }

    // start a new election by incrementing its term and initiating another round of Request-Vote RPCs
    pub(crate) fn on_election_timeout(
        &mut self,
        reply_to: &mut mpsc::Sender<Vec<(NodeId, RequestVoteRequest)>>,
    ) {
        if let Err(_) = self.common.persistent.update().current_term.increment() {
            // TODO crash as we cannot insrease term ? or remain a follower?
            // term is persistent so we cannot recover from crash automatically
            // neither receive append requests - every time any leader will be disqualified
        }
        // vote for self
        self.role.votes.checked_add(1).expect("too many votes");
        // multicast RequestVote
        let req = RequestVoteRequest {
            term: self.common.persistent.current_term,
            candidate_id: self.common.me.clone(),
            last_log_entry: self.common.persistent.log.last().map(|e| e.meta.clone()),
        };
        let requests = self
            .common
            .nodes
            .iter()
            .filter_map(|node_id| {
                (node_id != &self.common.me).then_some((node_id.clone(), req.clone()))
            })
            .collect();
        reply_to
            .try_send(requests)
            .expect("channel has a free slot");
    }

    // if a candidate (i.e. doesn't know the leader) receives a client request - it aborts the request
    pub(crate) fn on_client_request(
        &self,
        node_id: NodeId,
        _request: ClientRequest<C>,
        reply_to: &mut mpsc::Sender<(NodeId, ClientResponse)>,
    ) {
        reply_to
            .try_send((node_id, ClientResponse::Aborted))
            .expect("channel has a free slot");
    }
}

/// A server remains in follower state as long as
// it receives valid RPCs from a leader or candidate.
pub(crate) struct Follower {
    leader_id: Option<NodeId>,
}

impl<C: Clone> RaftNode<Follower, C> {
    pub(crate) fn new(
        persistent_state: Persistent<C>,
        persistence_tx: mpsc::UnboundedSender<Persistent<C>>,
        machine: mpsc::Sender<C>,
        nodes: Vec<NodeId>,
        me: NodeId,
    ) -> RaftNode<Follower, C> {
        Self {
            role: Follower { leader_id: None },
            common: CommonAttributes {
                persistent: Persistence::new(persistent_state, persistence_tx),
                commit_index: 0,
                last_applied: 0,
                nodes,
                me,
                machine,
            },
        }
    }

    pub(crate) fn on_append_request(
        &mut self,
        node_id: NodeId,
        request: AppendEntriesRequest<C>,
        reply_to: &mut mpsc::Sender<(NodeId, AppendEntriesResponse)>,
    ) {
        self.role.leader_id = Some(request.leader_id.clone());
        let (_, response) = self.process_append_request(request);
        // TODO apply to state machine
        // if commitIndex > lastApplied: increment lastApplied, apply
        // log[lastApplied] to state machine (§5.3)
        reply_to
            .try_send((node_id, response))
            .expect("channel has a free slot");
    }

    pub(crate) fn on_vote_request(
        &mut self,
        node_id: NodeId,
        request: RequestVoteRequest,
        reply_to: &mut mpsc::Sender<(NodeId, RequestVoteResponse)>,
    ) {
        let (_, response) = self.process_vote_request(request);
        reply_to
            .try_send((node_id, response))
            .expect("channel has a free slot");
    }

    pub(crate) fn on_election_timeout(
        self,
        reply_to: &mut mpsc::Sender<Vec<(NodeId, RequestVoteRequest)>>,
    ) -> RaftNode<Candidate, C> {
        let mut candidate: RaftNode<Candidate, C> = self.into();
        candidate.on_election_timeout(reply_to);
        candidate
    }

    // redirect to the leader
    pub(crate) fn on_client_request(
        &self,
        node_id: NodeId,
        _request: ClientRequest<C>,
        reply_to: &mut mpsc::Sender<(NodeId, ClientResponse)>,
    ) {
        let resp = if let Some(leader_id) = self.role.leader_id.clone() {
            ClientResponse::Redirect(leader_id)
        } else {
            ClientResponse::Aborted
        };
        reply_to
            .try_send((node_id, resp))
            .expect("channel has a free slot");
    }
}

/// State transitions

impl<C: Clone> From<RaftNode<Candidate, C>> for RaftNode<Follower, C> {
    fn from(candidate: RaftNode<Candidate, C>) -> Self {
        let RaftNode::<Candidate, C> { common, .. } = candidate;
        RaftNode::<Follower, C> {
            common,
            role: Follower { leader_id: None },
        }
    }
}

impl<C: Clone> From<RaftNode<Leader, C>> for RaftNode<Follower, C> {
    fn from(leader: RaftNode<Leader, C>) -> Self {
        let RaftNode::<Leader, C> { common, .. } = leader;
        RaftNode::<Follower, C> {
            common,
            role: Follower { leader_id: None },
        }
    }
}

impl<C: Clone> From<RaftNode<Follower, C>> for RaftNode<Candidate, C> {
    fn from(follower: RaftNode<Follower, C>) -> Self {
        let RaftNode::<Follower, C> { common, .. } = follower;
        RaftNode::<Candidate, C> {
            common,
            role: Candidate { votes: 0 },
        }
    }
}

impl<C: Clone> From<RaftNode<Candidate, C>> for RaftNode<Leader, C> {
    fn from(candidate: RaftNode<Candidate, C>) -> Self {
        let RaftNode::<Candidate, C> { common, .. } = candidate;
        let log_length = common.persistent.log.len();
        let next_index = common
            .nodes
            .iter()
            .map(|node_id| (node_id.clone(), log_length))
            .collect();
        let match_index = common
            .nodes
            .iter()
            .map(|node_id| (node_id.clone(), 0))
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
