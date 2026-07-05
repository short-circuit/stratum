import { useResponsive } from '../../lib/hooks/useResponsive';
import JournalPanelDesktop from './JournalPanel.desktop';
import JournalPanelMobile from './JournalPanel.mobile';

export default function JournalPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <JournalPanelMobile />;
  return <JournalPanelDesktop />;
}
