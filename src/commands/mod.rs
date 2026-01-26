//! CLI command implementations.
//!
//! Each command is implemented in its own submodule and uses MontContext
//! for all task graph operations.

mod check;
mod delete;
mod done;
mod init;
mod list;
pub mod llm;
mod ready;
pub mod shared;
mod show;
mod start;
mod status;
pub mod task_cmd;
pub mod unlock;

pub use check::check;
pub use delete::delete;
pub use done::done;
pub use init::init;
pub use list::list;
pub use llm::{claude, claude_ignore, claude_pre_validate, prompt};
pub use ready::ready;
pub use show::show;
pub use start::start;
pub use status::status;
pub use task_cmd::{distill, jot, task};
pub use unlock::unlock;
