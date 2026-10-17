//! Pure utility for grouping page paths into a folder tree for the
//! `pages_view = "tree"` sidebar/page-list rendering mode.
//!
//! Given a list of page paths it builds a nested `FolderNode` tree based only
//! on the path structure under `pages/`. It performs no I/O and imports no
//! React/DOM code so it can be unit-tested in isolation and reused by both the
//! sidebar (`PageTreeGrouped`) and the PagesHome views.

export interface FolderNode {
  /// Leaf: slug/name derived from filename; folder: folder name.
  name: string;
  /// Leaf only: the original page path (e.g. "pages/projects/note.md").
  fullPath?: string;
  isFolder: boolean;
  children: FolderNode[];
}

const PAGES_PREFIX = 'pages/';

interface TrieNode {
  children: Map<string, TrieNode>;
  /** Original full page path when this node is the terminal segment of a page. */
  leafPath?: string;
}

/** Strip a trailing `.md` from a leaf segment; leave other extensions as-is. */
function stripLeafExtension(segment: string): string {
  return segment.endsWith('.md') ? segment.slice(0, -'.md'.length) : segment;
}

/**
 * Group page paths into a folder tree.
 *
 * Only paths under `pages/` are considered; `journals/`, `notes/`, and any
 * other prefix are dropped. The `pages/` prefix itself is stripped so the root
 * of the returned tree contains entries directly inside `pages/` (there is no
 * `pages` wrapper node). Paths that leave nothing after the prefix are skipped.
 *
 * The returned tree is sorted depth-first with folder nodes before leaf nodes,
 * and alphabetical (via `localeCompare`) within each group. The input array is
 * not mutated; freshly-allocated arrays are returned.
 */
export function groupPagesByFolder(paths: string[]): FolderNode[] {
  const root: TrieNode = { children: new Map() };

  for (const path of paths) {
    if (!path.startsWith(PAGES_PREFIX)) continue;
    const rest = path.slice(PAGES_PREFIX.length);
    if (rest.length === 0) continue;
    const segments = rest.split('/');
    let node = root;
    for (const segment of segments) {
      let child = node.children.get(segment);
      if (!child) {
        child = { children: new Map() };
        node.children.set(segment, child);
      }
      node = child;
    }
    node.leafPath = path;
  }

  const build = (node: TrieNode): FolderNode[] => {
    const folders: FolderNode[] = [];
    const leaves: FolderNode[] = [];
    node.children.forEach((child, segment) => {
      const childNodes = build(child);
      // A node with descendants is a folder (and, if it also terminates a page,
      // additionally contributes a leaf for that path below).
      if (childNodes.length > 0) {
        folders.push({ name: segment, isFolder: true, children: childNodes });
      }
      if (child.leafPath !== undefined) {
        leaves.push({
          name: stripLeafExtension(segment),
          fullPath: child.leafPath,
          isFolder: false,
          children: [],
        });
      }
    });
    folders.sort((a, b) => a.name.localeCompare(b.name));
    leaves.sort((a, b) => a.name.localeCompare(b.name));
    return [...folders, ...leaves];
  };

  return build(root);
}
