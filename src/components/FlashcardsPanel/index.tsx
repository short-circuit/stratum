import { useResponsive } from '../../lib/hooks/useResponsive';
import FlashcardsPanelDesktop from './FlashcardsPanel.desktop';
import FlashcardsPanelMobile from './FlashcardsPanel.mobile';

export default function FlashcardsPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <FlashcardsPanelMobile />;
  return <FlashcardsPanelDesktop />;
}
