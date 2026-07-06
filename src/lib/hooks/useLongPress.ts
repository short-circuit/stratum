import { useRef, useCallback, useEffect } from 'react';

interface UseLongPressOptions {
  onLongPress: (e: React.TouchEvent | React.MouseEvent) => void;
  onClick?: (e: React.TouchEvent | React.MouseEvent) => void;
  threshold?: number;
}

interface UseLongPressHandlers {
  onTouchStart: (e: React.TouchEvent) => void;
  onTouchEnd: (e: React.TouchEvent) => void;
  onTouchMove: (e: React.TouchEvent) => void;
  onMouseDown: (e: React.MouseEvent) => void;
  onMouseUp: (e: React.MouseEvent) => void;
  onMouseMove: (e: React.MouseEvent) => void;
  onMouseLeave: (e: React.MouseEvent) => void;
}

export function useLongPress({
  onLongPress,
  onClick,
  threshold = 500,
}: UseLongPressOptions): UseLongPressHandlers {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isLongPressRef = useRef(false);

  // Keep callbacks in refs to avoid stale closures without recreating handlers.
  const callbacksRef = useRef({ onLongPress, onClick });
  useEffect(() => {
    callbacksRef.current = { onLongPress, onClick };
  }, [onLongPress, onClick]);

  const startTimer = useCallback((e: React.TouchEvent | React.MouseEvent) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    isLongPressRef.current = false;
    timerRef.current = setTimeout(() => {
      isLongPressRef.current = true;
      callbacksRef.current.onLongPress(e);
    }, threshold);
  }, [threshold]);

  const cancelTimer = useCallback((e: React.TouchEvent | React.MouseEvent) => {
    if (timerRef.current) { clearTimeout(timerRef.current); timerRef.current = null; }
    if (!isLongPressRef.current && callbacksRef.current.onClick) {
      callbacksRef.current.onClick(e);
    }
  }, []);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const onTouchStart = useCallback((e: React.TouchEvent) => startTimer(e), [startTimer]);
  const onTouchEnd = useCallback((e: React.TouchEvent) => cancelTimer(e), [cancelTimer]);
  const onTouchMove = useCallback(() => {
    if (timerRef.current) { clearTimeout(timerRef.current); timerRef.current = null; }
  }, []);
  const onMouseDown = useCallback((e: React.MouseEvent) => startTimer(e), [startTimer]);
  const onMouseUp = useCallback((e: React.MouseEvent) => cancelTimer(e), [cancelTimer]);
  const onMouseMove = useCallback(() => {
    if (timerRef.current) { clearTimeout(timerRef.current); timerRef.current = null; }
  }, []);
  const onMouseLeave = useCallback(() => {
    if (timerRef.current) { clearTimeout(timerRef.current); timerRef.current = null; }
  }, []);

  return {
    onTouchStart,
    onTouchEnd,
    onTouchMove,
    onMouseDown,
    onMouseUp,
    onMouseMove,
    onMouseLeave,
  };
}
