import { useState, useEffect } from 'react';
import { getPlatform } from '../platform';

const MOBILE_BREAKPOINT = 768;

export function useResponsive() {
  const [width, setWidth] = useState(typeof window !== 'undefined' ? window.innerWidth : 1200);

  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  const platform = getPlatform();

  // On Tauri mobile (Android/iOS), isMobile is always true
  // regardless of window width (WebView viewport may report desktop width)
  if (platform.isMobile) {
    return { isMobile: true, isDesktop: false, width };
  }

  return {
    isMobile: width < MOBILE_BREAKPOINT,
    isDesktop: width >= MOBILE_BREAKPOINT,
    width,
  };
}
