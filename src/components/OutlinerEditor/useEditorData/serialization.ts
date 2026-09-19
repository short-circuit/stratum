/**
 * Pure serialization helpers for the Outliner editor auto-save path.
 *
 * Split out of useEditorData.ts during the AGENTS.md sizing-gate followup
 * (E6.F1) so the hook module stays under the 500-line file gate. Contains no
 * React or DOM dependencies — everything needed to compute the deterministic
 * "what would be written to disk" key used to skip no-op saves (the ED-07
 * defense-in-depth check).
 */

import { blockNoteToDto, type BlockMeta } from '../dtoConverters';

export type BlockMetaRef = { current: Map<string, BlockMeta> };

/**
 * Deterministic key of what would be written to disk for a given document.
 * Computed through the same conversion the save path uses, so a stale/no-op
 * onChange compares equal and is skipped instead of rewriting the file with a
 * lossy backend round-trip (the ED-07 corruption vector).
 */
export function computeSerializedKey(
  blockNoteBlocks: any[],
  blockMeta: Map<string, BlockMeta>,
): string {
  const dtos = blockNoteToDto(structuredClone(blockNoteBlocks), blockMeta);
  return JSON.stringify(
    dtos.map((d) => [
      d.id,
      d.content,
      d.marker,
      d.priority,
      d.parent_id,
      d.left_id,
      d.heading_level,
      d.collapsed,
      ...(d.properties || [])
        .slice()
        .sort((a: [string, string], b: [string, string]) => a[0].localeCompare(b[0])),
    ]),
  );
}
