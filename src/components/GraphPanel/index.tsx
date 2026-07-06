import { useResponsive } from '../../lib/hooks/useResponsive';
import GraphPanelDesktop from './GraphPanel.desktop';
import GraphPanelMobile from './GraphPanel.mobile';

export default function GraphPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <GraphPanelMobile />;
  return <GraphPanelDesktop />;
}
