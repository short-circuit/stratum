//! Serde DTOs for the editor/service boundary.
//!
//! These structs are the seam to `stratum_app::block::BlockDto` and the frozen
//! React types in `src/lib/types.ts`. Field names, optionality and ordering are
//! part of the contract: changing a field here requires the same change in
//! `stratum-app` and `src/lib/types.ts`.
//!
//! `properties` is serialized as an array of `(key, value)` pairs (not a map) so
//! the wire shape matches the TypeScript `[string, string][]` type and the
//! `stratum-app` DTO byte-for-byte.
//!
//! Conversions are lossless for every field the DTO carries. The only known
//! asymmetry is intentional and lives above this layer: the React file-block
//! serializer drops the leading `!` from `![[assets/…]]`; the tests assert that
//! behaviour instead of hiding it.

use pkm_block::{Block, BlockId, BlockTree, Priority, TaskMarker};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wire-compatible block DTO.
///
/// This mirrors `stratum_app::block::BlockDto` exactly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockDto {
    pub id: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub left_id: Option<String>,
    pub properties: Vec<(String, String)>,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: bool,
    pub heading_level: Option<u8>,
}

/// Wire-compatible block list DTO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockListDto {
    pub blocks: Vec<BlockDto>,
}

/// A page document as handed to the editor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageDoc {
    pub path: String,
    pub title: Option<String>,
    pub blocks: Vec<BlockDto>,
}

/// Convert a DTO into a [`Block`].
///
/// An unparsable `id` falls back to a fresh UUID, exactly like
/// `stratum_app::block::dto_to_block`; this keeps legacy/corrupt rows editable
/// instead of rejecting the whole page. `parent_id` and `left_id` that fail to
/// parse become `None` (orphan roots) rather than aborting.
pub fn dto_to_block(dto: BlockDto) -> Block {
    let id = Uuid::parse_str(&dto.id).unwrap_or_else(|_| Uuid::new_v4());
    let mut block = Block::new(id, dto.content);
    block.parent_id = dto.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    block.left_id = dto.left_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    block.properties = dto.properties.into_iter().collect();
    block.marker = dto.marker.as_ref().and_then(|m| TaskMarker::parse(m));
    block.priority = dto.priority.as_ref().and_then(|p| Priority::parse(p));
    block.meta.collapsed = dto.collapsed;
    block.meta.heading_level = dto.heading_level;
    block
}

/// Convert every DTO into a [`Block`], preserving the input order.
pub fn dto_to_blocks(dtos: Vec<BlockDto>) -> Vec<Block> {
    dtos.into_iter().map(dto_to_block).collect()
}

/// Convert a [`Block`] into its wire DTO.
///
/// The `id` is always the canonical hyphenated UUID form, matching the
/// `stratum-app` serialization.
pub fn block_to_dto(block: &Block) -> BlockDto {
    BlockDto {
        id: block.id.to_string(),
        content: block.content.clone(),
        parent_id: block.parent_id.map(|id| id.to_string()),
        left_id: block.left_id.map(|id| id.to_string()),
        properties: block
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        marker: block.marker.map(|m| m.as_str().to_string()),
        priority: block.priority.map(|p| p.as_str().to_string()),
        collapsed: block.meta.collapsed,
        heading_level: block.meta.heading_level,
    }
}

/// Convert blocks into DTOs in the supplied order.
pub fn blocks_to_dto(blocks: &[Block]) -> Vec<BlockDto> {
    blocks.iter().map(block_to_dto).collect()
}

/// Build a [`PageDoc`] from a block tree, using the canonical document order.
pub fn page_doc_from_tree(
    path: impl Into<String>,
    title: Option<String>,
    tree: &BlockTree,
) -> PageDoc {
    let ordered: Vec<Block> = tree.depth_first_order().into_iter().cloned().collect();
    PageDoc {
        path: path.into(),
        title,
        blocks: blocks_to_dto(&ordered),
    }
}

/// Collect the block ids in a DTO list, ignoring unparsable ids.
pub fn dto_block_ids(dtos: &[BlockDto]) -> Vec<BlockId> {
    dtos.iter()
        .filter_map(|dto| Uuid::parse_str(&dto.id).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dto() -> BlockDto {
        BlockDto {
            id: "65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f".into(),
            content: "A task".into(),
            parent_id: None,
            left_id: None,
            properties: vec![("due".into(), "2026-10-15T07:00:00Z".into())],
            marker: Some("TODO".into()),
            priority: Some("A".into()),
            collapsed: true,
            heading_level: Some(2),
        }
    }

    #[test]
    fn dto_round_trip_preserves_every_field() {
        let dto = sample_dto();
        let block = dto_to_block(dto.clone());
        let back = block_to_dto(&block);
        assert_eq!(back, dto);
    }

    #[test]
    fn invalid_id_falls_back_to_a_fresh_uuid() {
        let mut dto = sample_dto();
        dto.id = "not-a-uuid".into();
        let block = dto_to_block(dto);
        assert_ne!(block.id.to_string(), "not-a-uuid");
    }

    #[test]
    fn unparsable_parent_and_left_become_none() {
        let mut dto = sample_dto();
        dto.parent_id = Some("bad".into());
        dto.left_id = Some("worse".into());
        let block = dto_to_block(dto);
        assert!(block.parent_id.is_none());
        assert!(block.left_id.is_none());
    }

    #[test]
    fn blocks_to_dto_preserves_order() {
        let a = Block::new(Uuid::new_v4(), "a".into());
        let b = Block::new(Uuid::new_v4(), "b".into());
        let dtos = blocks_to_dto(&[a.clone(), b.clone()]);
        assert_eq!(dtos[0].id, a.id.to_string());
        assert_eq!(dtos[1].id, b.id.to_string());
    }
}
