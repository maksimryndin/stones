#![forbid(unsafe_code)]
//#![cfg_attr(not(any(test, fuzzing)), deny(missing_docs))]

mod rpc;
mod state;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Term(u64);

struct NodeId;

type LogId = usize;
