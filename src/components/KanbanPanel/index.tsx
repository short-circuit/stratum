import { useResponsive } from '../../lib/hooks/useResponsive';
import KanbanPanelDesktop from './KanbanPanel.desktop';
import KanbanPanelMobile from './KanbanPanel.mobile';

export default function KanbanPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <KanbanPanelMobile />;
  return <KanbanPanelDesktop />;
}
