import { useResponsive } from '../../lib/hooks/useResponsive';
import TemplatesPanelDesktop from './TemplatesPanel.desktop';
import TemplatesPanelMobile from './TemplatesPanel.mobile';

export default function TemplatesPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <TemplatesPanelMobile />;
  return <TemplatesPanelDesktop />;
}
