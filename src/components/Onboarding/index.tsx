import { useResponsive } from '../../lib/hooks/useResponsive';
import OnboardingDesktop from './Onboarding.desktop';
import OnboardingMobile from './Onboarding.mobile';

export interface OnboardingProps {
  /** Called once the user finishes the wizard (disarms the first-run gate). */
  onDone: () => void;
}

export default function Onboarding({ onDone }: OnboardingProps) {
  const { isMobile } = useResponsive();
  if (isMobile) return <OnboardingMobile onDone={onDone} />;
  return <OnboardingDesktop onDone={onDone} />;
}
