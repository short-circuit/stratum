import { useResponsive } from '../../lib/hooks/useResponsive';
import RemindersPanelDesktop from './RemindersPanel.desktop';
import RemindersPanelMobile from './RemindersPanel.mobile';

export default function RemindersPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <RemindersPanelMobile />;
  return <RemindersPanelDesktop />;
}
