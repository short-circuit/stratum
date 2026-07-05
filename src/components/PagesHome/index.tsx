import { useResponsive } from '../../lib/hooks/useResponsive';
import PagesHomeDesktop from './PagesHome.desktop';
import PagesHomeMobile from './PagesHome.mobile';

export default function PagesHome() {
  const { isMobile } = useResponsive();

  if (isMobile) return <PagesHomeMobile />;
  return <PagesHomeDesktop />;
}
