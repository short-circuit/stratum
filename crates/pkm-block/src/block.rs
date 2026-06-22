use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub type BlockId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskMarker {
    Todo,
    Doing,
    Done,
    Now,
    Later,
    Waiting,
    Cancelled,
}

impl TaskMarker {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TODO" => Some(Self::Todo),
            "DOING" => Some(Self::Doing),
            "DONE" => Some(Self::Done),
            "NOW" => Some(Self::Now),
            "LATER" => Some(Self::Later),
            "WAITING" => Some(Self::Waiting),
            "CANCELLED" | "CANCELED" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Doing => "DOING",
            Self::Done => "DONE",
            Self::Now => "NOW",
            Self::Later => "LATER",
            Self::Waiting => "WAITING",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_open(&self) -> bool {
        matches!(self, Self::Todo | Self::Doing | Self::Now | Self::Later | Self::Waiting)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    A,
    B,
    C,
}

impl Priority {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "A" => Some(Self::A),
            "B" => Some(Self::B),
            "C" => Some(Self::C),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockMeta {
    pub collapsed: bool,
    pub heading_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub content: String,
    pub parent_id: Option<BlockId>,
    pub left_id: Option<BlockId>,
    pub properties: BTreeMap<String, String>,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub meta: BlockMeta,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Block {
    pub fn new(id: BlockId, content: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            content,
            parent_id: None,
            left_id: None,
            properties: BTreeMap::new(),
            marker: None,
            priority: None,
            meta: BlockMeta::default(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn with_parent(mut self, parent_id: BlockId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_left(mut self, left_id: BlockId) -> Self {
        self.left_id = Some(left_id);
        self
    }

    pub fn with_marker(mut self, marker: TaskMarker) -> Self {
        self.marker = Some(marker);
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn is_task(&self) -> bool {
        self.marker.is_some()
    }

    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    pub fn is_heading(&self) -> bool {
        self.meta.heading_level.is_some()
    }

    pub fn has_children_in(&self, tree: &super::tree::BlockTree) -> bool {
        tree.first_child(self.id).is_some()
    }
}

/// Link types extracted from block content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockLink {
    PageRef {
        target: String,
        display: Option<String>,
    },
    BlockRef {
        target_id: BlockId,
        display: Option<String>,
    },
    Tag {
        name: String,
    },
    ExternalUrl {
        url: String,
        display: Option<String>,
    },
}

/// An embed within block content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Embed {
    Block { target_id: BlockId },
    Page { target_page: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_marker_from_str() {
        assert_eq!(TaskMarker::from_str("TODO"), Some(TaskMarker::Todo));
        assert_eq!(TaskMarker::from_str("done"), Some(TaskMarker::Done));
        assert_eq!(TaskMarker::from_str("LATER"), Some(TaskMarker::Later));
        assert_eq!(TaskMarker::from_str("cancelled"), Some(TaskMarker::Cancelled));
        assert_eq!(TaskMarker::from_str("canceled"), Some(TaskMarker::Cancelled));
        assert_eq!(TaskMarker::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_task_marker_open_closed() {
        assert!(TaskMarker::Todo.is_open());
        assert!(TaskMarker::Doing.is_open());
        assert!(TaskMarker::Now.is_open());
        assert!(!TaskMarker::Done.is_open());
        assert!(TaskMarker::Done.is_closed());
        assert!(TaskMarker::Cancelled.is_closed());
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!(Priority::from_str("A"), Some(Priority::A));
        assert_eq!(Priority::from_str("b"), Some(Priority::B));
        assert_eq!(Priority::from_str("D"), None);
    }

    #[test]
    fn test_block_creation() {
        let id = Uuid::new_v4();
        let block = Block::new(id, "Hello world".into());
        assert_eq!(block.id, id);
        assert_eq!(block.content, "Hello world");
        assert!(block.is_root());
        assert!(!block.is_task());
        assert!(block.properties.is_empty());
    }

    #[test]
    fn test_block_builder() {
        let id = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let block = Block::new(id, "A task".into())
            .with_parent(parent)
            .with_marker(TaskMarker::Todo)
            .with_priority(Priority::A)
            .with_property("deadline", "2026-07-01");

        assert_eq!(block.parent_id, Some(parent));
        assert_eq!(block.marker, Some(TaskMarker::Todo));
        assert_eq!(block.priority, Some(Priority::A));
        assert_eq!(block.properties.get("deadline").map(|s| s.as_str()), Some("2026-07-01"));
        assert!(block.is_task());
    }

    #[test]
    fn test_block_meta_default() {
        let meta = BlockMeta::default();
        assert!(!meta.collapsed);
        assert!(meta.heading_level.is_none());
    }

    #[test]
    fn test_marker_as_str_roundtrip() {
        for marker in &[
            TaskMarker::Todo,
            TaskMarker::Doing,
            TaskMarker::Done,
            TaskMarker::Now,
            TaskMarker::Later,
            TaskMarker::Waiting,
            TaskMarker::Cancelled,
        ] {
            assert_eq!(TaskMarker::from_str(marker.as_str()), Some(*marker));
        }
    }

    #[test]
    fn test_block_heading() {
        let id = Uuid::new_v4();
        let mut block = Block::new(id, "# Heading".into());
        assert!(!block.is_heading());
        block.meta.heading_level = Some(1);
        assert!(block.is_heading());
    }
}
