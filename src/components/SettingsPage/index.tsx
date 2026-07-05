import { useResponsive } from '../../lib/hooks/useResponsive';
import SettingsPageDesktop from './SettingsPage.desktop';
import SettingsPageMobile from './SettingsPage.mobile';

export default function SettingsPage() {
  const { isMobile } = useResponsive();
  if (isMobile) return <SettingsPageMobile />;
  return <SettingsPageDesktop />;
}
