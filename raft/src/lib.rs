#![forbid(unsafe_code)]
//#![cfg_attr(not(any(test, fuzzing)), deny(missing_docs))]

mod client;
mod effects;
mod entry;
mod protocol;
mod rpc;
mod state;

pub use client::{ClientRequest, ClientResponse};
pub use entry::{Entry, EntryMeta};
pub use protocol::{main, IncomingMessage, OutgoingMessage};
pub use rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
pub use state::Persistent;

/// Time is divided into terms, and each term begins
/// with an election.
/// Election Safety: at most one leader can be elected in a
/// given term.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Term(u64);

struct TermOverflow;

impl Term {
    pub(crate) fn increment(&mut self) -> Result<(), TermOverflow> {
        self.0 = self.0.checked_add(1).ok_or(TermOverflow)?;
        Ok(())
    }
}

pub type LogId = usize;
