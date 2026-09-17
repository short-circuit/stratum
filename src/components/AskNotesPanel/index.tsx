import { useResponsive } from '../../lib/hooks/useResponsive';
import AskNotesPanelDesktop from './AskNotesPanel.desktop';
import AskNotesPanelMobile from './AskNotesPanel.mobile';

export default function AskNotesPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <AskNotesPanelMobile />;
  return <AskNotesPanelDesktop />;
}
