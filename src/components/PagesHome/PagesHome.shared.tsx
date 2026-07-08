import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { useStore } from '../../stores/appStore';
import type { PageDto, VaultInfo } from '../../lib/types';

export interface PagesHomeData {
  pages: PageDto[];
  vault: VaultInfo | null;
  navigateToPage: (path: string) => void;
}

export function usePagesHomeData(): PagesHomeData {
  const { pages, vault } = useStore(useShallow(
    s => ({ pages: s.pages, vault: s.vault }),
  ));
  const navigate = useNavigate();

  const navigateToPage = (path: string) => {
    navigate(`/page/${encodeURIComponent(path)}`);
  };

  return { pages, vault, navigateToPage };
}
