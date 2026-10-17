import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { useStore } from '../../stores/appStore';
import { useSettingsStore } from '../../stores/settingsStore';
import type { PageDto, VaultInfo } from '../../lib/types';

export interface PagesHomeData {
  pages: PageDto[];
  vault: VaultInfo | null;
  pagesView: string;
  navigateToPage: (path: string) => void;
}

export function usePagesHomeData(): PagesHomeData {
  const { pages, vault } = useStore(useShallow(
    s => ({ pages: s.pages, vault: s.vault }),
  ));
  const settings = useSettingsStore(s => s.settings);
  const navigate = useNavigate();

  const navigateToPage = (path: string) => {
    navigate(`/page/${encodeURIComponent(path)}`);
  };

  return { pages, vault, pagesView: settings?.layout?.pages_view ?? 'flat', navigateToPage };
}
