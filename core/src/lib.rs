#![forbid(unsafe_code)]
//#![cfg_attr(not(any(test, fuzzing)), deny(missing_docs))]
//

pub mod machine;
pub mod node;

pub use machine::StateMachine;
pub use node::NodeId;
