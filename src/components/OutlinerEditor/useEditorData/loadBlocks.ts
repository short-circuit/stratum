/**
 * useBlockLoading — backend block load, DTO conversion, and marker extraction
 * for the Outliner editor.
 *
 * Extracted verbatim from useEditorData/index.ts during the t_dc58c24c
 * sizing-gate decomposition. `onLoaded` is invoked at the point the original
 * owner bumped its metaVersion counter. No JSX.
 *
 * @module OutlinerEditor/useEditorData/loadBlocks
 */

import { useEffect } from 'react';
import type { Dispatch, MutableRefObject, SetStateAction } from 'react';
import { normalizeContent } from '../../../lib/wikiLinks';
import * as api from '../../../lib/commands';
import { dtoToBlockNote, type BlockMeta } from '../dtoConverters';
import { computeSerializedKey } from './serialization';
import type { EditorData } from './types';

interface UseBlockLoadingArgs {
  pagePath: string;
  editor: EditorData['editor'];
  blockMetaRef: MutableRefObject<Map<string, BlockMeta>>;
  loadedSnapshotRef: MutableRefObject<string | null>;
  isProcessingRef: MutableRefObject<boolean>;
  setStatus: Dispatch<SetStateAction<string>>;
  setError: Dispatch<SetStateAction<string | null>>;
  setPageMarkers: Dispatch<SetStateAction<string[]>>;
  onLoaded: () => void;
}

/**
 * Loads block DTOs for a page from the Rust backend, converts them to BlockNote's
 * internal format, extracts the page's markers, and moves the editor to the
 * `ready` state. Suppresses the onChange fired by replaceBlocks so a programmatic
 * load never triggers a spurious auto-save (ED-07).
 */
export function useBlockLoading({
  pagePath,
  editor,
  blockMetaRef,
  loadedSnapshotRef,
  isProcessingRef,
  setStatus,
  setError,
  setPageMarkers,
  onLoaded,
}: UseBlockLoadingArgs) {
  useEffect(() => {
    api
      .getBlocks(pagePath)
      .then(({ blocks }) => {
        try {
          blockMetaRef.current.clear();
          let bnBlocks: any[] = [];
          for (const b of blocks) b.content = normalizeContent(b.content);
          bnBlocks = dtoToBlockNote(blocks, blockMetaRef.current);
          if (bnBlocks.length > 0) {
            // Suppress the onChange fired by replaceBlocks so the programmatic
            // load does not trigger an auto-save that would rewrite the file
            // before the user has typed anything (ED-07: autosave-on-load).
            isProcessingRef.current = true;
            try {
              editor.replaceBlocks(editor.document, bnBlocks);
            } finally {
              isProcessingRef.current = false;
            }
          }
          // Snapshot the post-load editor state (through the same conversion the
          // save path uses) so auto-save skip logic can compare exact equality.
          try {
            loadedSnapshotRef.current = computeSerializedKey(editor.document, blockMetaRef.current);
          } catch {
            loadedSnapshotRef.current = null;
          }
          const markers = [
            ...new Set(
              blocks.filter((b) => b.marker).map((b) => b.marker!),
            ),
          ];
          setPageMarkers(markers);
          setStatus('ready');
          onLoaded();
        } catch (e) {
          console.error('[OutlinerEditor] replaceBlocks failed:', e);
          setError(String(e));
          setStatus('error');
        }
      })
      .catch((err) => {
        console.error('[OutlinerEditor] getBlocks failed:', err);
        setError(String(err));
        setStatus('error');
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- blockMetaRef/loadedSnapshotRef/isProcessingRef are stable refs and setStatus/setError/setPageMarkers/onLoaded are parent-owned; onLoaded deliberately omitted so the load effect never re-fires on re-render.
  }, [pagePath, editor]);
}
