import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import OutlinerEditor from './OutlinerEditor';
import BacklinksPanel from './BacklinksPanel';
import PropertiesPanel from './PropertiesPanel';

export default function PageView() {
  const { pagePath } = useParams<{ pagePath: string }>();
  const { currentPage, openPage } = useStore();
  const [editorKey, setEditorKey] = useState(0);

  useEffect(() => {
    if (pagePath) {
      openPage(decodeURIComponent(pagePath));
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setEditorKey(k => k + 1);
    }
  }, [pagePath]);

  if (!pagePath) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--secondary-400)]">
        <div className="text-center">
          <h2 className="text-xl font-semibold mb-2">Stratum PKM</h2>
          <p className="text-sm">Select a page from the sidebar or create a new one.</p>
        </div>
      </div>
    );
  }

  if (!currentPage) {
    return <div className="p-4 text-[var(--secondary-400)]">Loading...</div>;
  }

  return (
    <div className="h-full flex flex-col">
      {/* Page header */}
      <div className="px-6 py-3 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] flex items-center gap-3">
        <h1 className="text-lg font-bold">
          {currentPage.title || currentPage.slug}
        </h1>
        <span className="text-xs text-[var(--secondary-400)]">
          {currentPage.path}
        </span>
      </div>

      {/* Editor + Backlinks */}
      <div className="flex-1 overflow-auto">
        <OutlinerEditor key={editorKey} pagePath={currentPage.path} />
      </div>

      {/* Backlinks dock */}
      <BacklinksPanel pagePath={currentPage.path} />

      {/* Properties panel */}
      <PropertiesPanel pagePath={currentPage.path} />
    </div>
  );
}
