import { useResponsive } from '../lib/hooks/useResponsive';
import VoiceCaptureButtonDesktop from './VoiceCaptureButton.desktop';
import VoiceCaptureButtonMobile from './VoiceCaptureButton.mobile';
import type { VoiceCaptureButtonProps } from './VoiceCaptureButton.desktop';

/** One-tap voice-capture button; renders the mobile FAB or desktop header control. */
export default function VoiceCaptureButton(props: VoiceCaptureButtonProps) {
  const { isMobile } = useResponsive();
  if (isMobile) return <VoiceCaptureButtonMobile {...props} />;
  return <VoiceCaptureButtonDesktop {...props} />;
}
