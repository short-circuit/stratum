//! Block DTO conversion between `stratum-app` and the editor engine.
//!
//! The two DTOs are field-identical; keeping the conversion in one module makes
//! a future field addition fail to compile in exactly one place.

use stratum_editor::BlockDto;

/// Convert an `stratum-app` block DTO into the editor DTO.
pub(crate) fn app_block_to_editor(dto: stratum_app::block::BlockDto) -> BlockDto {
    BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
    }
}

/// Convert an editor DTO into the `stratum-app` block DTO.
pub(crate) fn editor_block_to_app(dto: BlockDto) -> stratum_app::block::BlockDto {
    stratum_app::block::BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
    }
}
