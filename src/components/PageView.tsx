import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import type { BlockDto } from '../lib/types';
import * as api from '../lib/commands';
import BlockEditor from './BlockEditor';

export default function PageView() {
  const { pagePath } = useParams<{ pagePath: string }>();
  const { currentPage, openPage } = useStore();
  const [blocks, setBlocks] = useState<BlockDto[]>([]);

  useEffect(() => {
    if (pagePath) {
      openPage(decodeURIComponent(pagePath));
    }
  }, [pagePath]);

  useEffect(() => {
    if (currentPage) {
      api.getBlocks(currentPage.path).then(({ blocks: b }) => setBlocks(b));
    }
  }, [currentPage]);

  if (!pagePath) {
    return (
      <div className="flex items-center justify-center h-full text-gray-400">
        <div className="text-center">
          <h2 className="text-xl font-semibold mb-2">Stratum PKM</h2>
          <p className="text-sm">Select a page from the sidebar or create a new one.</p>
        </div>
      </div>
    );
  }

  if (!currentPage) {
    return <div className="p-4 text-gray-400">Loading...</div>;
  }

  return (
    <div className="h-full flex flex-col">
      {/* Page header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700">
        <h1 className="text-xl font-bold">
          {currentPage.title || currentPage.slug}
        </h1>
        <p className="text-xs text-gray-500 mt-1">
          {currentPage.path} · {currentPage.block_count} blocks
        </p>
      </div>

      {/* Blocks */}
      <div className="flex-1 overflow-auto p-4">
        <BlockEditor
          pagePath={currentPage.path}
          blocks={blocks}
          onBlocksChange={setBlocks}
        />
      </div>
    </div>
  );
}
