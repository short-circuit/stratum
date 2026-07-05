import { useResponsive } from '../../lib/hooks/useResponsive';
import BacklinksPanelDesktop from './BacklinksPanel.desktop';
import BacklinksPanelMobile from './BacklinksPanel.mobile';
import type { BacklinksPanelProps } from './BacklinksPanel.shared';

export default function BacklinksPanel(props: BacklinksPanelProps) {
  const { isMobile } = useResponsive();
  if (isMobile) return <BacklinksPanelMobile {...props} />;
  return <BacklinksPanelDesktop {...props} />;
}
