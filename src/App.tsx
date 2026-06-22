import { useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useStore } from './stores/appStore';
import Sidebar from './components/Sidebar';
import PageView from './components/PageView';
import JournalPanel from './components/JournalPanel';
import PagesHome from './components/PagesHome';
import SearchPanel from './components/SearchPanel';
import QueryPanel from './components/QueryPanel';
import TemplatesPanel from './components/TemplatesPanel';
import FlashcardsPanel from './components/FlashcardsPanel';
import WhiteboardPanel from './components/WhiteboardPanel';
import SettingsPage from './components/SettingsPage';

export default function App() {
  const { loadVault, loadPages, error } = useStore();

  useEffect(() => {
    loadVault();
    loadPages();
  }, []);

  return (
    <div className="flex h-screen w-screen bg-white dark:bg-[var(--secondary-900)] text-[var(--secondary-900)] dark:text-[var(--secondary-100)]">
      <Sidebar />
      <main className="flex-1 overflow-auto">
        {error && (
          <div className="bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 p-2 text-sm">
            {error}
          </div>
        )}
        <Routes>
          <Route path="/" element={<PagesHome />} />
          <Route path="/journal" element={<JournalPanel />} />
          <Route path="/page/:pagePath" element={<PageView />} />
          <Route path="/search" element={<SearchPanel />} />
          <Route path="/query" element={<QueryPanel />} />
          <Route path="/templates" element={<TemplatesPanel />} />
          <Route path="/flashcards" element={<FlashcardsPanel />} />
          <Route path="/whiteboards" element={<WhiteboardPanel />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}
