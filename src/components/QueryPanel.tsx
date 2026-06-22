import { useState } from 'react';
import * as api from '../lib/commands';

export default function QueryPanel() {
  const [datalog, setDatalog] = useState(
    '{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}'
  );
  const [result, setResult] = useState<{ columns: string[]; rows: string[][] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  const doQuery = async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await api.runQuery(datalog);
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="p-4 max-w-4xl mx-auto">
      <h2 className="text-lg font-semibold mb-3">Datalog Query</h2>

      <textarea
        value={datalog}
        onChange={e => setDatalog(e.target.value)}
        className="w-full h-32 px-3 py-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm font-mono"
        placeholder="Enter Datalog query..."
      />

      <div className="flex gap-2 mt-2 mb-4">
        <button
          onClick={doQuery}
          disabled={running}
          className="px-4 py-2 bg-[var(--accent-500)] text-white rounded text-sm hover:bg-[var(--accent-600)] disabled:opacity-50"
        >
          {running ? 'Running...' : 'Run Query'}
        </button>
        <button
          onClick={() => setDatalog('{:query [:find ?b :where [?b :block/marker "TODO"]]}')}
          className="px-3 py-2 text-sm text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
        >
          Reset
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 dark:bg-red-900/30 text-red-600 dark:text-red-300 rounded text-sm mb-3">
          {error}
        </div>
      )}

      {result && result.rows.length > 0 && (
        <div className="overflow-auto">
          <table className="w-full text-sm border-collapse">
            <thead>
              <tr>
                {result.columns.map((col, i) => (
                  <th key={i} className="text-left px-3 py-2 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                    {col}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {result.rows.map((row, i) => (
                <tr key={i} className="border-b border-gray-100 dark:border-gray-800">
                  {row.map((cell, j) => (
                    <td key={j} className="px-3 py-2">{cell}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {result && result.rows.length === 0 && (
        <p className="text-gray-400 text-sm">Query returned no results.</p>
      )}
    </div>
  );
}
