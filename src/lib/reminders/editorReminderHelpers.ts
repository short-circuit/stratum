//! Shared reminder UI helpers for the block editor.
//!
//! These utilities are used by both the desktop and mobile OutlinerEditor
//! variants to (a) read a block's `due` property from its BlockMeta and
//! (b) attach a delegated click handler that opens the ReminderDialog when a
//! reminder chip is tapped. Keeping them here avoids duplicating the identical
//! DOM-delegation and property-lookup logic across platform variants.

import type { BlockMeta } from '../../components/OutlinerEditor/dtoConverters';

/**
 * Return the `due` property value for a block, or `null` if the block has
 * either no BlockMeta entry or no `due` property set.
 */
export function getBlockDue(
  blockId: string,
  metaByBlock: Map<string, BlockMeta>,
): string | null {
  const meta = metaByBlock.get(blockId);
  if (!meta) return null;
  const entry = meta.properties.find(([key]) => key === 'due');
  return entry ? entry[1] : null;
}

/**
 * Attach a delegated click listener to the editor container that resolves the
 * containing block of any tapped reminder chip and invokes `onOpenBlock` with
 * that block's id.
 *
 * Returns a cleanup function that detaches the listener.
 */
export function attachReminderChipClick(
  container: HTMLElement,
  onOpenBlock: (blockId: string) => void,
): () => void {
  const handler = (e: MouseEvent) => {
    const target = e.target as HTMLElement;
    const chip = target.closest?.('[data-reminder-chip]');
    if (!chip) return;
    // A chip itself may not carry the block id — walk up to the block element.
    const blockEl = target.closest('[data-block-id]');
    const blockId = blockEl?.getAttribute('data-block-id');
    if (!blockId) return;
    onOpenBlock(blockId);
  };
  container.addEventListener('click', handler);
  return () => container.removeEventListener('click', handler);
}
