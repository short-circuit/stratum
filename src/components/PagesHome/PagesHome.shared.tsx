import { useNavigate } from 'react-router-dom';
import { useStore } from '../../stores/appStore';
import type { PageDto, VaultInfo } from '../../lib/types';

export interface PagesHomeData {
  pages: PageDto[];
  vault: VaultInfo | null;
  navigateToPage: (path: string) => void;
}

export function usePagesHomeData(): PagesHomeData {
  const { pages, vault } = useStore();
  const navigate = useNavigate();

  const navigateToPage = (path: string) => {
    navigate(`/page/${encodeURIComponent(path)}`);
  };

  return { pages, vault, navigateToPage };
}
