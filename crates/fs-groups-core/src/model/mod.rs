//! Value types of the groups contract.
//!
//! Pattern: Value objects — plain data, one file per concept; no storage logic.

mod delegation;
mod delivery;
mod group;
mod invitation;
mod member;
mod nesting_mode;
mod origin;
mod page;
mod parent_request;
mod right_flow;
mod role;
mod source;

pub use delegation::{Delegation, Right};
pub use delivery::{Delivery, DeliveryState, Offer};
pub use group::{Group, NS};
pub use invitation::{Invitation, InvitationState};
pub use member::{EffectiveMember, Member, Membership};
pub use nesting_mode::NestingMode;
pub use origin::{EffectiveOrigin, Origin};
pub use page::{Cursor, Page};
pub use parent_request::{ParentRequest, RequestKind, RequestState};
pub use right_flow::RightFlow;
pub use role::Role;
pub use source::{Source, SourceState};
