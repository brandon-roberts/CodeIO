//! codeio-router — routing & indexing engine (match + route + index ONLY; no code writing).
//!
//! Tier-normalized per docs/architecture-model.md, each tier usable in base form by other modules:
//!   L0  `selector`  — pure, total matching over IR nodes. No policy, no I/O. (mechanism)
//!   L1  `router`    — maps matches to destinations via relative/dynamic rules. (capability)
//!   L2  `index`     — bidirectional IR<->location map + replayable trace. (experience/coherence)
//!
//! Each tier depends only on the tier below and on codeio-ir. This is the styling engine too:
//! a "style rule" is a selector routed to a render bucket instead of a language stack.

pub mod selector;
pub mod router;
pub mod index;

pub use selector::{Selector, SelectorPart};
pub use router::{Route, RoutingRule, Router};
pub use index::{RouteIndex, TraceStep};
