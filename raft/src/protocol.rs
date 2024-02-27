use crate::effects::generate_timeout;
use crate::state::{Candidate, Follower, Leader, RaftNode, Transition};
use futures::stream::{self, StreamExt};
use futures::FutureExt;

enum CurrentRole<C> {
    Leader(RaftNode<Leader, C>),
    Candidate(RaftNode<Candidate, C>),
    Follower(RaftNode<Follower, C>),
}

// Protocol is validated by types and method signatures at state.rs
pub async fn main<C>() {
    let mut current = CurrentRole::Follower(RaftNode::<Follower, C>::new().await);
    let election_timeout = generate_timeout().fuse();
    futures::pin_mut!(election_timeout);
    let mut append_requests = stream::iter(vec![]).fuse();
    let mut append_requests_responses = stream::iter(vec![]).fuse();
    let mut vote_requests = stream::iter(vec![]).fuse();
    let mut vote_request_responses = stream::iter(vec![]).fuse();
    let mut client_requests = stream::iter(vec![]).fuse();

    // on any transition to Follower - generate a new election timeout
    loop {
        // biased for deterministic execution
        futures::select_biased! {
            req = &mut append_requests.next() => {
                let (conninfo, req) = req.unwrap();
                let (reply_to, receiver) = oneshot::channel();
                match current {
                    CurrentRole::Follower(mut follower) => {
                        follower.on_append_request(req, reply_to).await;
                        current = CurrentRole::Follower(follower);
                    },
                    CurrentRole::Candidate(candidate) => {
                        match candidate.on_append_request(req, reply_to).await {
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
                let response = receiver.await;
                //transport.unicast(conninfo, response)
            },
            response = &mut append_requests_responses.next() => {
                let response = response.unwrap();
                match current {
                    CurrentRole::Follower(follower) => {
                        current = CurrentRole::Follower(follower);
                    },
                    CurrentRole::Candidate(candidate) => {
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Leader(leader) => {
                        match leader.on_append_reponse(response).await {
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
            req = &mut vote_requests.next() => {
                let (conninfo, req) = req.unwrap();
                let (reply_to, receiver) = oneshot::channel();
                match current {
                    CurrentRole::Follower(mut follower) => {
                        follower.on_vote_request(req, reply_to).await;
                        current = CurrentRole::Follower(follower);
                    },
                    CurrentRole::Candidate(candidate) => {
                        match candidate.on_vote_request(req, reply_to).await {
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
                let response = receiver.await;
                //transport.unicast(conninfo, response)
            },
            response = &mut vote_request_responses.next() => {
                let response = response.unwrap();
                match current {
                    CurrentRole::Follower(follower) => {
                        current = CurrentRole::Follower(follower);
                    },
                    CurrentRole::Candidate(candidate) => {
                        match candidate.on_vote_response(response).await {
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
            },
            timeout = &mut election_timeout =>
                match current {
                    CurrentRole::Follower(follower) => {
                        election_timeout.as_mut().set(generate_timeout().fuse());
                        let candidate = follower.on_election_timeout().await;
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Candidate(mut candidate) => {
                        election_timeout.as_mut().set(generate_timeout().fuse());
                        candidate.on_election_timeout().await;
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Leader(leader) => {
                        current = CurrentRole::Leader(leader);
                    }
                },
            // leader broadcast
            req = &mut client_requests.next() => {
                let (conninfo, req) = req.unwrap();
                let (reply_to, receiver) = oneshot::channel();
                match current {
                    CurrentRole::Follower(follower) => {
                        follower.on_client_request(req, reply_to).await;
                        current = CurrentRole::Follower(follower);
                    },
                    CurrentRole::Candidate(candidate) => {
                        candidate.on_client_request(req, reply_to).await;
                        current = CurrentRole::Candidate(candidate);
                    },
                    CurrentRole::Leader(mut leader) => {
                        leader.on_client_request(req, reply_to).await;
                        current = CurrentRole::Leader(leader);
                    }
                }
                let response = receiver.await;
            },
            default => todo!("leader's broadcast; handle  graceful shutdown; nodes reconfiguration"),
        };
    }
}
