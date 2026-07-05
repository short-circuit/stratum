import { useResponsive } from '../../lib/hooks/useResponsive';
import PageViewDesktop from './desktop';
import PageViewMobile from './mobile';

export default function PageView() {
  const { isMobile } = useResponsive();
  if (isMobile) return <PageViewMobile />;
  return <PageViewDesktop />;
}
