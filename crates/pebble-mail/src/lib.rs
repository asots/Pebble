pub mod imap;
pub mod parser;
pub mod sync;
pub mod thread;

pub use imap::{ImapConfig, ImapProvider, SmtpConfig};
pub use sync::{SyncConfig, SyncWorker};
