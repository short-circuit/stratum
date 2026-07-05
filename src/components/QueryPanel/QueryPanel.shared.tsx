import { useState } from 'react';
import * as api from '../../lib/commands';

export interface QueryResult {
  columns: string[];
  rows: string[][];
}

export function useDatalogQuery() {
  const [datalog, setDatalog] = useState(
    '{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}'
  );
  const [result, setResult] = useState<QueryResult | null>(null);
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

  const resetQuery = () => {
    setDatalog('{:query [:find ?b :where [?b :block/marker "TODO"]]}');
  };

  return { datalog, setDatalog, result, error, running, doQuery, resetQuery };
}
