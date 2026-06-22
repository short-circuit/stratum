import { useState, useEffect } from 'react';
import * as api from '../lib/commands';

export default function TemplatesPanel() {
  const [templates, setTemplates] = useState<any[]>([]);
  const [targetPath, setTargetPath] = useState('');
  const [variables, setVariables] = useState<[string, string][]>([['', '']]);
  const [message, setMessage] = useState('');

  useEffect(() => {
    api.listTemplates().then(setTemplates).catch(console.error);
  }, []);

  const apply = async (name: string) => {
    if (!targetPath) return;
    try {
      const vars = variables.filter(([k]) => k.trim());
      await api.applyTemplate(name, targetPath, vars);
      setMessage(`Created: ${targetPath}`);
      setTargetPath('');
    } catch (e) {
      setMessage(`Error: ${e}`);
    }
  };

  return (
    <div className="p-4 max-w-2xl mx-auto">
      <h2 className="text-lg font-semibold mb-3">Templates</h2>

      <div className="mb-4">
        <label className="text-xs text-gray-500 block mb-1">Target page path</label>
        <input
          type="text"
          value={targetPath}
          onChange={e => setTargetPath(e.target.value)}
          placeholder="pages/my-new-page.md"
          className="w-full text-sm px-2 py-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
        />
      </div>

      <div className="mb-4">
        <label className="text-xs text-gray-500 block mb-1">Variables</label>
        {variables.map(([k, v], i) => (
          <div key={i} className="flex gap-1 mb-1">
            <input
              type="text"
              value={k}
              onChange={e => {
                const next = [...variables];
                next[i] = [e.target.value, v];
                setVariables(next);
              }}
              placeholder="key"
              className="flex-1 text-xs px-1.5 py-0.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
            />
            <input
              type="text"
              value={v}
              onChange={e => {
                const next = [...variables];
                next[i] = [k, e.target.value];
                setVariables(next);
              }}
              placeholder="value"
              className="flex-1 text-xs px-1.5 py-0.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
            />
          </div>
        ))}
        <button
          onClick={() => setVariables([...variables, ['', '']])}
          className="text-xs text-blue-500 hover:text-blue-600"
        >
          + Add variable
        </button>
      </div>

      {message && (
        <div className="text-sm mb-3 p-2 bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded">
          {message}
        </div>
      )}

      <div className="space-y-2">
        {templates.map(t => (
          <div
            key={t.name}
            className="p-3 rounded border border-gray-200 dark:border-gray-700 hover:border-blue-300"
          >
            <div className="flex items-center justify-between mb-1">
              <h3 className="text-sm font-medium">{t.name}</h3>
              <button
                onClick={() => apply(t.name)}
                className="text-xs px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                Apply
              </button>
            </div>
            {t.description && (
              <p className="text-xs text-gray-500">{t.description}</p>
            )}
          </div>
        ))}
        {templates.length === 0 && (
          <p className="text-sm text-gray-400">
            No templates yet. Save a page as template to get started.
          </p>
        )}
      </div>
    </div>
  );
}
