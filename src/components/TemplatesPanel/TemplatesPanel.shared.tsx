import { useState, useEffect, useCallback } from 'react';
import * as api from '../../lib/commands';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export interface TemplateDto {
  name: string;
  description?: string;
  [key: string]: any;
}

export function useTemplates() {
  const [templates, setTemplates] = useState<TemplateDto[]>([]);
  const [targetPath, setTargetPath] = useState('');
  const [variables, setVariables] = useState<[string, string][]>([['', '']]);
  const [message, setMessage] = useState('');

  const loadTemplates = useCallback(async () => {
    try {
      const data = await api.listTemplates();
      setTemplates(data);
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const apply = useCallback(
    async (name: string) => {
      if (!targetPath) return;
      try {
        const vars = variables.filter(([k]) => k.trim());
        await api.applyTemplate(name, targetPath, vars);
        setMessage(`Created: ${targetPath}`);
        setTargetPath('');
      } catch (e) {
        setMessage(`Error: ${e}`);
      }
    },
    [targetPath, variables],
  );

  const addVariableRow = useCallback(() => {
    setVariables((prev) => [...prev, ['', '']]);
  }, []);

  const updateVariableKey = useCallback((index: number, key: string) => {
    setVariables((prev) => {
      const next = [...prev];
      next[index] = [key, prev[index][1]];
      return next;
    });
  }, []);

  const updateVariableValue = useCallback((index: number, value: string) => {
    setVariables((prev) => {
      const next = [...prev];
      next[index] = [prev[index][0], value];
      return next;
    });
  }, []);

  return {
    templates,
    targetPath,
    setTargetPath,
    variables,
    message,
    setMessage,
    apply,
    addVariableRow,
    updateVariableKey,
    updateVariableValue,
    loadTemplates,
  };
}
