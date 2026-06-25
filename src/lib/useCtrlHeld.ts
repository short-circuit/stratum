import { useEffect, useRef } from 'react';

export function useCtrlHeld(): { ctrlHeld: React.MutableRefObject<boolean> } {
  const ctrlHeld = useRef(false);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') {
        ctrlHeld.current = true;
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') {
        ctrlHeld.current = false;
      }
    };

    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
    };
  }, []);

  return { ctrlHeld };
}
