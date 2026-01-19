//! CLI command implementations.
//!
//! Each command is implemented in its own submodule and uses MontContext
//! for all task graph operations.

mod check;
mod delete;
mod distill;
mod done;
pub mod edit;
pub mod jot;
mod list;
pub mod llm;
pub mod new;
mod ready;
pub mod shared;
mod show;
mod start;
mod status;
pub mod unlock;

pub use check::check;
pub use delete::delete;
pub use distill::distill;
pub use done::done;
pub use edit::edit;
pub use jot::jot;
pub use list::list;
pub use llm::{claude, prompt};
pub use new::new;
pub use ready::ready;
pub use show::show;
pub use start::start;
pub use status::status;
pub use unlock::unlock;
