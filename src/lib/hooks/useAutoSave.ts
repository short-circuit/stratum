import { useRef, useCallback, useState } from 'react';

export function useAutoSave(saveFn: () => Promise<void>, delay = 500) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [saving, setSaving] = useState(false);

  const scheduleSave = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(async () => {
      setSaving(true);
      try { await saveFn(); } finally { setSaving(false); }
    }, delay);
  }, [saveFn, delay]);

  const flush = useCallback(async () => {
    if (timerRef.current) { clearTimeout(timerRef.current); timerRef.current = null; }
    setSaving(true);
    try { await saveFn(); } finally { setSaving(false); }
  }, [saveFn]);

  return { scheduleSave, flush, saving };
}
