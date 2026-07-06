import { useResponsive } from '../../lib/hooks/useResponsive';
import QueryPanelDesktop from './QueryPanel.desktop';
import QueryPanelMobile from './QueryPanel.mobile';

export default function QueryPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <QueryPanelMobile />;
  return <QueryPanelDesktop />;
}
