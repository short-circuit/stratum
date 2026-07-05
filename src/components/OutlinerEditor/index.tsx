/**
 * Platform router for the OutlinerEditor.
 *
 * Delegates to the desktop or mobile variant based on the current
 * responsive breakpoint and platform (Tauri native mobile vs browser).
 *
 * @module OutlinerEditor/index
 */

import { useResponsive } from '../../lib/hooks/useResponsive';
import OutlinerEditorDesktop from './OutlinerEditor.desktop';
import OutlinerEditorMobile from './OutlinerEditor.mobile';
import type { Props } from './OutlinerEditor.shared';

export default function OutlinerEditor(props: Props) {
  const { isMobile } = useResponsive();

  if (isMobile) {
    return <OutlinerEditorMobile {...props} />;
  }

  return <OutlinerEditorDesktop {...props} />;
}
