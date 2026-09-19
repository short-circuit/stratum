import { describe, it, expect, afterEach } from 'vitest';
import { getPlatform } from './platform';

function setUa(ua: string) {
  Object.defineProperty(window.navigator, 'userAgent', {
    value: ua,
    configurable: true,
  });
}

afterEach(() => {
  delete (window as any).__TAURI_PLATFORM__;
  delete (window as any).__TAURI_INTERNALS__;
  setUa('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36');
});

describe('getPlatform', () => {
  it('detects Android from user agent (Tauri 2.x does not inject platform global)', () => {
    setUa('Mozilla/5.0 (Linux; Android 14; sdk_gphone64_x86_64 Build/UE1A.230829.050; wv) AppleWebKit/537.36');
    const p = getPlatform();
    expect(p.isMobile).toBe(true);
    expect(p.platform).toBe('mobile');
    expect(p.os).toBe('android');
  });

  it('detects iOS from user agent', () => {
    setUa('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15');
    const p = getPlatform();
    expect(p.isMobile).toBe(true);
    expect(p.os).toBe('ios');
  });

  it('detects desktop Linux/Windows/macOS user agents as desktop', () => {
    setUa('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36');
    expect(getPlatform().isMobile).toBe(false);
    setUa('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36');
    expect(getPlatform().isMobile).toBe(false);
    setUa('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15');
    expect(getPlatform().isMobile).toBe(false);
  });

  it('honors an injected __TAURI_PLATFORM__ global over the user agent', () => {
    (window as any).__TAURI_PLATFORM__ = 'linux';
    setUa('Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36');
    const p = getPlatform();
    expect(p.os).toBe('linux');
    expect(p.isMobile).toBe(false);
  });

  it('markes isTauri only when a Tauri marker is present', () => {
    expect(getPlatform().isTauri).toBe(false);
    (window as any).__TAURI_INTERNALS__ = { metadata: {} };
    expect(getPlatform().isTauri).toBe(true);
  });
});
