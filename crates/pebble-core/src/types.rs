use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub provider: ProviderType,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Imap,
    Gmail,
    Outlook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub account_id: String,
    pub remote_id: String,
    pub name: String,
    pub folder_type: FolderType,
    pub role: Option<FolderRole>,
    pub parent_id: Option<String>,
    pub color: Option<String>,
    pub is_system: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FolderType {
    Folder,
    Label,
    Category,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FolderRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Archive,
    Spam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub account_id: String,
    pub remote_id: String,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub thread_id: Option<String>,
    pub subject: String,
    pub snippet: String,
    pub from_address: String,
    pub from_name: String,
    pub to_list: Vec<EmailAddress>,
    pub cc_list: Vec<EmailAddress>,
    pub bcc_list: Vec<EmailAddress>,
    pub body_text: String,
    pub body_html_raw: String,
    pub has_attachments: bool,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_draft: bool,
    pub date: i64,
    pub remote_version: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub name: Option<String>,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub message_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLabel {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_system: bool,
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KanbanColumn {
    Todo,
    Waiting,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanCard {
    pub message_id: String,
    pub column: KanbanColumn,
    pub position: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnoozedMessage {
    pub message_id: String,
    pub snoozed_at: i64,
    pub unsnoozed_at: i64,
    pub return_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedSender {
    pub account_id: String,
    pub email: String,
    pub trust_type: TrustType,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrustType {
    Images,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub conditions: String,
    pub actions: String,
    pub is_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyMode {
    Strict,
    TrustSender(String),
    LoadOnce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedHtml {
    pub html: String,
    pub trackers_blocked: Vec<TrackerInfo>,
    pub images_blocked: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerInfo {
    pub domain: String,
    pub tracker_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub has_labels: bool,
    pub has_folders: bool,
    pub has_categories: bool,
    pub has_push: bool,
    pub has_threads: bool,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
