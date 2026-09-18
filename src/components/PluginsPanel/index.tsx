import { useResponsive } from '../../lib/hooks/useResponsive';
import PluginsPanelDesktop from './PluginsPanel.desktop';
import PluginsPanelMobile from './PluginsPanel.mobile';

export default function PluginsPanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <PluginsPanelMobile />;
  return <PluginsPanelDesktop />;
}
