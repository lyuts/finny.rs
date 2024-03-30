//! The builder-style API structures for defining your Finny FSM. The procedural macro parses
//! these method calls and generated the optimized implementation.

mod event;
mod fsm;
mod state;
mod sub;

pub use self::event::*;
pub use self::fsm::*;
pub use self::state::*;
pub use self::sub::*;

#[cfg(feature = "std")]
pub type FsmQueueMock<F> = crate::FsmEventQueueVec<F>;

#[cfg(not(feature = "std"))]
pub type FsmQueueMock<F> = crate::FsmEventQueueNull<F>;
