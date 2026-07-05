import { useResponsive } from '../../lib/hooks/useResponsive';
import WhiteboardPanelDesktop from './WhiteboardPanel.desktop';
import WhiteboardPanelMobile from './WhiteboardPanel.mobile';

export default function WhiteboardPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <WhiteboardPanelMobile />;
  return <WhiteboardPanelDesktop />;
}
