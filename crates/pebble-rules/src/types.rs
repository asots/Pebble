use serde::{Deserialize, Serialize};
use pebble_core::KanbanColumn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConditionSet {
    pub operator: LogicalOp,
    pub conditions: Vec<RuleCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: ConditionField,
    pub op: ConditionOp,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionField {
    From,
    To,
    Subject,
    Body,
    HasAttachment,
    Domain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    Contains,
    NotContains,
    Equals,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RuleAction {
    AddLabel(String),
    MoveToFolder(String),
    MarkRead,
    Archive,
    SetKanbanColumn(KanbanColumn),
}
