use crate::client::{ClientRequest, ClientResponse};
use crate::effects::generate_timeout;
use crate::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use crate::state::{Candidate, Follower, Leader, RaftNode, Transition};
use futures::channel::mpsc;
use futures::stream::StreamExt;
use futures::FutureExt;
use stones_core::{NodeId, StateMachine};

enum CurrentRole<C, S: StateMachine<C>> {
    Leader(RaftNode<Leader, C, S>),
    Candidate(RaftNode<Candidate, C, S>),
    Follower(RaftNode<Follower, C, S>),
}

enum IncomingMessage<C> {
    Client((NodeId, ClientRequest<C>)),
    Append((NodeId, AppendEntriesRequest<C>)),
    Vote((NodeId, RequestVoteRequest)),
    AppendResponse((NodeId, AppendEntriesResponse)),
    VoteResponse((NodeId, RequestVoteResponse)),
}

enum OutgoingMessage<C> {
    Client((NodeId, ClientResponse)),
    Append(Vec<(NodeId, AppendEntriesRequest<C>)>),
    Vote(Vec<(NodeId, RequestVoteRequest)>),
    AppendResponse((NodeId, AppendEntriesResponse)),
    VoteResponse((NodeId, RequestVoteResponse)),
}

// Protocol is validated by types and method signatures at state.rs
pub async fn main<C, S>()
where
    S: StateMachine<C>,
{
    let mut current = CurrentRole::Follower(RaftNode::<Follower, C, S>::new());
    let election_timeout = generate_timeout().fuse();
    futures::pin_mut!(election_timeout);

    // external channels
    //let (outgoing_tx, outgoing_rx) = mpsc::unbounded();
    let (incoming_tx, mut incoming_rx) = mpsc::unbounded(); // incoming_tx - is populated by transport

    let (mut outgoing_client_tx, mut outgoing_client_rx) = mpsc::channel(1);
    let (mut outgoing_append_tx, mut outgoing_append_rx) = mpsc::channel(1);
    let (mut outgoing_vote_tx, mut outgoing_vote_rx) = mpsc::channel(1);
    let (mut outgoing_append_resp_tx, mut outgoing_append_resp_rx) = mpsc::channel(1);
    let (mut outgoing_vote_resp_tx, mut outgoing_vote_resp_append_rx) = mpsc::channel(1);

    // on any transition to Follower - generate a new election timeout
    loop {
        // biased for deterministic execution
        futures::select_biased! {
            // TODO add persistence layer - is the first on every cycle - we should save the state prior to any outgoing activity!
            // We should drain outgoing messages first to prevent accumulating them
            // out = &mut outgoing_rx.next() => {
            //     //transport.send(out).await; // TODO or try_send??
            // },
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

            default => todo!("handle  graceful shutdown; nodes reconfiguration"),
        };
    }
}
