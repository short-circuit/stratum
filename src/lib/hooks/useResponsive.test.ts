import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import { useResponsive } from './useResponsive';

// Mock getPlatform to return non-mobile by default
vi.mock('../platform', () => ({
  getPlatform: () => ({ isMobile: false }),
}));

describe('useResponsive', () => {
  const originalInnerWidth = window.innerWidth;

  afterEach(() => {
    window.innerWidth = originalInnerWidth;
  });

  it('detects desktop by default (width >= 768)', () => {
    window.innerWidth = 1200;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isDesktop).toBe(true);
    expect(result.current.isMobile).toBe(false);
  });

  it('detects mobile breakpoint (width < 768)', () => {
    window.innerWidth = 375;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isDesktop).toBe(false);
    expect(result.current.isMobile).toBe(true);
  });

  it('returns current width', () => {
    window.innerWidth = 1024;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.width).toBe(1024);
  });

  it('reacts to window resize', () => {
    window.innerWidth = 1200;
    const { result } = renderHook(() => useResponsive());

    act(() => {
      window.innerWidth = 600;
      window.dispatchEvent(new Event('resize'));
    });

    expect(result.current.isMobile).toBe(true);
    expect(result.current.width).toBe(600);
  });
});
