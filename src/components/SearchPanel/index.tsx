import { useResponsive } from '../../lib/hooks/useResponsive';
import SearchPanelDesktop from './SearchPanel.desktop';
import SearchPanelMobile from './SearchPanel.mobile';

export default function SearchPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <SearchPanelMobile />;
  return <SearchPanelDesktop />;
}
