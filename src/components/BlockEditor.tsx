import { useState } from 'react';
import * as api from '../lib/commands';
import type { BlockDto } from '../lib/types';

interface Props {
  pagePath: string;
  blocks: BlockDto[];
  onBlocksChange: (blocks: BlockDto[]) => void;
}

export default function BlockEditor({ pagePath, blocks, onBlocksChange }: Props) {
  const [newContent, setNewContent] = useState('');

  const addBlock = async () => {
    if (!newContent.trim()) return;
    const block = await api.insertBlock(pagePath, newContent, null, null);
    onBlocksChange([...blocks, block]);
    setNewContent('');
  };

  const updateContent = async (block: BlockDto, content: string) => {
    const updated = { ...block, content };
    await api.updateBlock(pagePath, updated);
    onBlocksChange(blocks.map(b => b.id === block.id ? updated : b));
  };

  const removeBlock = async (blockId: string) => {
    await api.deleteBlock(blockId);
    onBlocksChange(blocks.filter(b => b.id !== blockId));
  };

  return (
    <div className="space-y-2 max-w-3xl">
      {blocks.map(block => (
        <div
          key={block.id}
          className="group flex items-start gap-2 p-2 rounded hover:bg-gray-50 dark:hover:bg-gray-800/50 border border-transparent hover:border-gray-200 dark:hover:border-gray-700"
        >
          {/* Bullet / Marker */}
          <div className="mt-1.5 flex items-center gap-1">
            {block.marker && (
              <span className={`text-xs px-1 rounded ${
                block.marker === 'DONE' ? 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300' :
                block.marker === 'DOING' ? 'bg-yellow-100 dark:bg-yellow-900 text-yellow-700 dark:text-yellow-300' :
                'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300'
              }`}>
                {block.marker}
              </span>
            )}
            <span className="text-gray-400 select-none">•</span>
          </div>

          {/* Content */}
          <input
            type="text"
            value={block.content}
            onChange={e => updateContent(block, e.target.value)}
            className="flex-1 bg-transparent border-none outline-none text-sm"
            placeholder="Type something..."
          />

          {/* Actions */}
          <button
            onClick={() => removeBlock(block.id)}
            className="opacity-0 group-hover:opacity-100 text-xs text-red-400 hover:text-red-600 px-1"
          >
            ×
          </button>
        </div>
      ))}

      {/* New block input */}
      <div className="flex items-start gap-2 p-2">
        <span className="text-gray-400 select-none mt-1.5">•</span>
        <input
          type="text"
          value={newContent}
          onChange={e => setNewContent(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter') addBlock();
          }}
          className="flex-1 bg-transparent border border-gray-200 dark:border-gray-700 rounded px-2 py-1 text-sm outline-none focus:border-blue-400"
          placeholder="New block... (Enter to add)"
        />
      </div>
    </div>
  );
}
