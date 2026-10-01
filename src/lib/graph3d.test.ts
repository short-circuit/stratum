import { describe, it, expect, afterEach } from 'vitest';
import { detectGraph3dSupport } from './graph3d';

function setHardware(memory: number | undefined, cores: number | undefined) {
  Object.defineProperty(navigator, 'deviceMemory', {
    value: memory,
    configurable: true,
  });
  Object.defineProperty(navigator, 'hardwareConcurrency', {
    value: cores,
    configurable: true,
  });
}

afterEach(() => {
  // Restore real jsdom behavior (no WebGL context).
  HTMLCanvasElement.prototype.getContext = vi.fn(() => null) as any;
  setHardware(undefined, undefined);
});

describe('detectGraph3dSupport', () => {
  it('reports no-webgl when a WebGL context cannot be created', () => {
    HTMLCanvasElement.prototype.getContext = vi.fn(() => null) as any;
    setHardware(8, 8);
    const support = detectGraph3dSupport();
    expect(support.supported).toBe(false);
    expect(support.reason).toBe('no-webgl');
  });

  it('reports ok when WebGL is available on a capable device', () => {
    HTMLCanvasElement.prototype.getContext = vi.fn((name: string) =>
      name === 'webgl2' || name === 'webgl' || name === 'experimental-webgl' ? {} : null,
    ) as any;
    setHardware(8, 8);
    const support = detectGraph3dSupport();
    expect(support.supported).toBe(true);
    expect(support.reason).toBe('ok');
  });

  it('reports low-end when the device has <= 2GB RAM despite WebGL', () => {
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({})) as any;
    setHardware(2, 4);
    const support = detectGraph3dSupport();
    expect(support.supported).toBe(false);
    expect(support.reason).toBe('low-end');
  });

  it('reports low-end when the device has <= 2 cores', () => {
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({})) as any;
    setHardware(8, 1);
    const support = detectGraph3dSupport();
    expect(support.supported).toBe(false);
    expect(support.reason).toBe('low-end');
  });

  it('treats an unexpected exception as unsupported (graceful fallback)', () => {
    HTMLCanvasElement.prototype.getContext = vi.fn(() => {
      throw new Error('webgl crash');
    }) as any;
    setHardware(8, 8);
    const support = detectGraph3dSupport();
    expect(support.supported).toBe(false);
  });
});
