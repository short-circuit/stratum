import { useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useStore } from './stores/appStore';
import Sidebar from './components/Sidebar';
import PageView from './components/PageView';
import JournalPanel from './components/JournalPanel';
import SearchPanel from './components/SearchPanel';
import QueryPanel from './components/QueryPanel';

export default function App() {
  const { loadVault, loadPages, error } = useStore();

  useEffect(() => {
    loadVault();
    loadPages();
  }, []);

  return (
    <div className="flex h-screen w-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <Sidebar />
      <main className="flex-1 overflow-auto">
        {error && (
          <div className="bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 p-2 text-sm">
            {error}
          </div>
        )}
        <Routes>
          <Route path="/" element={<JournalPanel />} />
          <Route path="/journal" element={<JournalPanel />} />
          <Route path="/page/:pagePath" element={<PageView />} />
          <Route path="/search" element={<SearchPanel />} />
          <Route path="/query" element={<QueryPanel />} />
        </Routes>
      </main>
    </div>
  );
}
