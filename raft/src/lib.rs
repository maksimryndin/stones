#![forbid(unsafe_code)]
//#![cfg_attr(not(any(test, fuzzing)), deny(missing_docs))]

mod rpc;
mod state;

pub use state::main;

/// Time is divided into terms, and each term begins
/// with an election.
/// Election Safety: at most one leader can be elected in a
/// given term.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Term(u64);

struct NodeId;

type LogId = usize;
