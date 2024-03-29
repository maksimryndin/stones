use crate::client::{ClientRequest, ClientResponse};
use crate::effects::generate_timeout;
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crate::state::{Candidate, Follower, Leader, Persistent, RaftNode, Transition};
use futures::channel::mpsc;
use futures::stream::StreamExt;
use futures::FutureExt;
use stones_core::{NodeId, PersistenceLayer, StateMachine};

enum CurrentRole<C: Clone> {
    Leader(RaftNode<Leader, C>),
    Candidate(RaftNode<Candidate, C>),
    Follower(RaftNode<Follower, C>),
}

pub enum IncomingMessage<C: Clone> {
    Client((NodeId, ClientRequest<C>)),
    Append((NodeId, AppendEntriesRequest<C>)),
    Vote((NodeId, RequestVoteRequest)),
    AppendResponse((NodeId, AppendEntriesResponse)),
    VoteResponse((NodeId, RequestVoteResponse)),
}

pub enum OutgoingMessage<C: Clone> {
    Client((NodeId, ClientResponse)),
    Append(Vec<(NodeId, AppendEntriesRequest<C>)>),
    Vote(Vec<(NodeId, RequestVoteRequest)>),
    AppendResponse((NodeId, AppendEntriesResponse)),
    VoteResponse((NodeId, RequestVoteResponse)),
}

// Protocol is validated by types and method signatures at state.rs
// Assumptions about a transport
// - reliable (either delivered of failed)
// - retries failed deliveries
pub async fn main<C, S, P>(
    me: NodeId,
    mut machine: S,
    mut persistence_layer: P,
    outgoing_tx: mpsc::UnboundedSender<OutgoingMessage<C>>,
    mut incoming_rx: mpsc::UnboundedReceiver<IncomingMessage<C>>,
) where
    S: StateMachine<C>,
    P: PersistenceLayer<Persistent<C>>,
    C: Clone,
{
    let (persistence_tx, mut persistence_rx) = mpsc::unbounded();
    let (machine_tx, mut machine_rx) = mpsc::channel(1);
    let (mut outgoing_client_tx, mut outgoing_client_rx) = mpsc::channel(1);
    let (mut outgoing_append_tx, mut outgoing_append_rx) = mpsc::channel(1);
    let (mut outgoing_vote_tx, mut outgoing_vote_rx) = mpsc::channel(1);
    let (mut outgoing_append_resp_tx, mut outgoing_append_resp_rx) = mpsc::channel(1);
    let (mut outgoing_vote_resp_tx, mut outgoing_vote_resp_append_rx) = mpsc::channel(1);

    let persistent = persistence_layer.load();
    let nodes = vec![]; // TODO read from configuration
    let mut current = CurrentRole::Follower(RaftNode::<Follower, C>::new(
        persistent,
        persistence_tx,
        machine_tx,
        nodes,
        me,
    ));
    let election_timeout = generate_timeout().fuse();
    futures::pin_mut!(election_timeout);

    // on any transition to Follower - generate a new election timeout
    // encode on Type transition that we should call election timeout??
    // Persistent should wrap all relevant fields - check derefmut
    loop {
        // biased for deterministic execution and
        // clear priority of different effects
        futures::select_biased! {
            // first, on every cycle we should try apply changes to the state machine!
            to_apply = &mut machine_rx.next() => {
                let to_apply = to_apply.unwrap();
                machine.apply(to_apply);
            },
            // second, on every cycle we should save the state prior to any outgoing activity!
            to_save = &mut persistence_rx.next() => {
                // last update wins
                let mut to_save = to_save.unwrap();
                while let Ok(Some(upd)) = persistence_rx.try_next() {
                    to_save = upd;
                }
                persistence_layer.save(to_save).unwrap();
            },
            // third, drain outgoing messages to prevent accumulating them
            out = &mut outgoing_client_rx.next() => {
                let out = out.unwrap();
                outgoing_tx.unbounded_send(OutgoingMessage::<C>::Client(out)).expect("transport is not closed");
            },
            out = &mut outgoing_append_rx.next() => {
                let out = out.unwrap();
                outgoing_tx.unbounded_send(OutgoingMessage::<C>::Append(out)).expect("transport is not closed");
            },
            out = &mut outgoing_vote_rx.next() => {
                let out = out.unwrap();
                outgoing_tx.unbounded_send(OutgoingMessage::<C>::Vote(out)).expect("transport is not closed");
            },
            out = &mut outgoing_append_resp_rx.next() => {
                let out = out.unwrap();
                outgoing_tx.unbounded_send(OutgoingMessage::<C>::AppendResponse(out)).expect("transport is not closed");
            },
            out = &mut outgoing_vote_resp_append_rx.next() => {
                let out = out.unwrap();
                outgoing_tx.unbounded_send(OutgoingMessage::<C>::VoteResponse(out)).expect("transport is not closed");
            },
            // TODO leader's heartbeat
            // Upon election: send initial empty AppendEntries RPCs
            // (heartbeat) to each server; repeat during idle periods to
            // prevent election timeouts (§5.2)
            // handle incoming messages
            req = &mut incoming_rx.next() => {
                if req.is_none() {
                    break;
                }
                let req = req.unwrap();
                match req {
                    IncomingMessage::Client((node_id, req)) => {
                        match current {
                            CurrentRole::Follower(follower) => {
                                follower.on_client_request(node_id, req, &mut outgoing_client_tx);
                                current = CurrentRole::Follower(follower);
                            },
                            CurrentRole::Candidate(candidate) => {
                                candidate.on_client_request(node_id, req, &mut outgoing_client_tx);
                                current = CurrentRole::Candidate(candidate);
                            },
                            CurrentRole::Leader(mut leader) => {
                                leader.on_client_request(node_id, req, &mut outgoing_append_tx);
                                current = CurrentRole::Leader(leader);
                            }
                        }
                    },
                    IncomingMessage::Append((node_id, req)) => {
                        match current {
                            CurrentRole::Follower(mut follower) => {
                                follower.on_append_request(node_id, req, &mut outgoing_append_resp_tx);
                                current = CurrentRole::Follower(follower);
                            },
                            CurrentRole::Candidate(candidate) => {
                                match candidate.on_append_request(node_id, req, &mut outgoing_append_resp_tx) {
                                    Transition::Remains(candidate) => {
                                        current = CurrentRole::Candidate(candidate);
                                    },
                                    Transition::ChangedTo(follower) => {
                                        election_timeout.as_mut().set(generate_timeout().fuse());
                                        current = CurrentRole::Follower(follower);
                                    },
                                }
                            },
                            CurrentRole::Leader(leader) => {
                                current = CurrentRole::Leader(leader);
                                continue;
                            }
                        }
                    },
                    IncomingMessage::Vote((node_id, req)) => {
                        match current {
                            CurrentRole::Follower(mut follower) => {
                                follower.on_vote_request(node_id, req, &mut outgoing_vote_resp_tx);
                                current = CurrentRole::Follower(follower);
                            },
                            CurrentRole::Candidate(candidate) => {
                                match candidate.on_vote_request(node_id, req, &mut outgoing_vote_resp_tx) {
                                    Transition::Remains(candidate) => {
                                        current = CurrentRole::Candidate(candidate);
                                    },
                                    Transition::ChangedTo(follower) => {
                                        election_timeout.as_mut().set(generate_timeout().fuse());
                                        current = CurrentRole::Follower(follower);
                                    },
                                }
                            },
                            CurrentRole::Leader(leader) => {
                                current = CurrentRole::Leader(leader);
                                continue;
                            }
                        }
                    },
                    IncomingMessage::AppendResponse((node_id, response)) => {
                        match current {
                            CurrentRole::Follower(follower) => {
                                current = CurrentRole::Follower(follower);
                            },
                            CurrentRole::Candidate(candidate) => {
                                current = CurrentRole::Candidate(candidate);
                            },
                            CurrentRole::Leader(leader) => {
                                match leader.on_append_reponse(node_id, response, &mut outgoing_client_tx, &mut outgoing_append_tx) {
                                    Transition::Remains(leader) => {
                                        current = CurrentRole::Leader(leader);
                                    },
                                    Transition::ChangedTo(follower) => {
                                        election_timeout.as_mut().set(generate_timeout().fuse());
                                        current = CurrentRole::Follower(follower);
                                    }
                                }
                            }
                        }
                    },
                    IncomingMessage::VoteResponse((node_id, response)) => {
                        match current {
                            CurrentRole::Follower(follower) => {
                                current = CurrentRole::Follower(follower);
                            },
                            CurrentRole::Candidate(candidate) => {
                                match candidate.on_vote_response(node_id, response) {
                                    Transition::Remains(candidate) => {
                                        match candidate.check_votes() {
                                            Transition::Remains(candidate) => {
                                                current = CurrentRole::Candidate(candidate);
                                            },
                                            Transition::ChangedTo(leader) => {
                                                current = CurrentRole::Leader(leader);
                                            },
                                        }
                                    },
                                    Transition::ChangedTo(follower) => {
                                        election_timeout.as_mut().set(generate_timeout().fuse());
                                        current = CurrentRole::Follower(follower);
                                    },
                                }
                            },
                            CurrentRole::Leader(leader) => {
                                current = CurrentRole::Leader(leader);
                            }
                        }
                    }
                }
            },
            // handle timeout if nothing has triggered before
            timeout = &mut election_timeout =>
                match current {
                    CurrentRole::Follower(follower) => {
                        election_timeout.as_mut().set(generate_timeout().fuse());
                        let candidate = follower.on_election_timeout(&mut outgoing_vote_tx);
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Candidate(mut candidate) => {
                        election_timeout.as_mut().set(generate_timeout().fuse());
                        candidate.on_election_timeout(&mut outgoing_vote_tx);
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Leader(leader) => {
                        current = CurrentRole::Leader(leader);
                    }
                },

            // TODO handle  graceful shutdown; nodes reconfiguration
        };
    }
}
