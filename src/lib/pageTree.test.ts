import { describe, it, expect } from 'vitest';
import { groupPagesByFolder } from './pageTree';

describe('groupPagesByFolder', () => {
  it('builds a folder tree from pages/ paths and drops non-pages paths', () => {
    const result = groupPagesByFolder([
      'pages/a.md',
      'pages/projects/b.md',
      'pages/projects/sub/c.md',
      'journals/2026-10-14.md',
      'notes/loose.md',
    ]);

    expect(result).toEqual([
      {
        name: 'projects',
        isFolder: true,
        children: [
          {
            name: 'sub',
            isFolder: true,
            children: [
              { name: 'c', fullPath: 'pages/projects/sub/c.md', isFolder: false, children: [] },
            ],
          },
          { name: 'b', fullPath: 'pages/projects/b.md', isFolder: false, children: [] },
        ],
      },
      { name: 'a', fullPath: 'pages/a.md', isFolder: false, children: [] },
    ]);
  });

  it('orders folder nodes before leaf nodes, alphabetical within each group', () => {
    const result = groupPagesByFolder(['pages/z.md', 'pages/a.md', 'pages/m/leaf.md', 'pages/b.md']);
    // folders first (m), then leaves (a, b, z) — each group alphabetical.
    expect(result.map(n => n.name)).toEqual(['m', 'a', 'b', 'z']);
    expect(result[0].isFolder).toBe(true);
    expect(result.slice(1).every(n => !n.isFolder)).toBe(true);
  });

  it('supports arbitrary nesting depth', () => {
    const result = groupPagesByFolder(['pages/x/y/z/deep.md']);
    const x = result[0];
    expect(x.name).toBe('x');
    expect(x.isFolder).toBe(true);
    const y = x.children[0];
    expect(y.name).toBe('y');
    expect(y.isFolder).toBe(true);
    const z = y.children[0];
    expect(z.name).toBe('z');
    expect(z.isFolder).toBe(true);
    expect(z.children[0]).toEqual({
      name: 'deep',
      fullPath: 'pages/x/y/z/deep.md',
      isFolder: false,
      children: [],
    });
  });

  it('leaves non-`.md` extension segments as-is in the leaf name', () => {
    const result = groupPagesByFolder(['pages/notes/todo.txt']);
    expect(result).toEqual([
      {
        name: 'notes',
        isFolder: true,
        children: [{ name: 'todo.txt', fullPath: 'pages/notes/todo.txt', isFolder: false, children: [] }],
      },
    ]);
  });

  it('returns [] when only non-pages paths are given', () => {
    expect(groupPagesByFolder(['journals/2026-10-14.md', 'notes/loose.md'])).toEqual([]);
    expect(groupPagesByFolder([])).toEqual([]);
  });

  it('does not mutate the input array', () => {
    const input = ['pages/a.md', 'pages/b.md', 'journals/x.md'];
    const copy = [...input];
    groupPagesByFolder(input);
    expect(input).toEqual(copy);
  });
});
