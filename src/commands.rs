//! Commands module — stubs for all forge subcommands.

pub mod health;
pub mod cd;
pub mod check;
pub mod create;
pub mod edit;
pub mod help;
pub mod include;
pub mod lang;
pub mod list;
pub mod open;
pub mod overseer;
pub mod overseer_def;
pub mod pick;
pub mod remove;
pub mod session;
pub mod setup;
pub mod sync;

pub use cd::run as cd;
pub use health::run as health;
pub use cd::run as cd;
pub use check::run as check;
pub use create::run as create;
pub use edit::run as edit;
pub use include::run as include;
pub use lang::run as lang;
pub use list::run as list;
pub use open::run as open;
pub use overseer::run as overseer;
pub use overseer_def::run as overseer_def;
pub use pick::run as pick;
pub use remove::run as remove;
pub use session::run as session;
pub use setup::run as setup;
pub use sync::run as sync;