#![forbid(unsafe_code)]
//#![cfg_attr(not(any(test, fuzzing)), deny(missing_docs))]

mod effects;
mod entry;
mod protocol;
mod rpc;
mod state;

pub use protocol::main;

/// Time is divided into terms, and each term begins
/// with an election.
/// Election Safety: at most one leader can be elected in a
/// given term.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct Term(u64);

struct TermOverflow;

impl Term {
    pub(crate) fn increment(&mut self) -> Result<(), TermOverflow> {
        self.0 = self.0.checked_add(1).ok_or(TermOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct NodeId(String);

type LogId = usize;
