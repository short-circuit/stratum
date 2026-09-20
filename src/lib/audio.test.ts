import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  plainTextFromContent,
  textFromBlock,
  speakText,
} from './audio';

// These tests exercise the real `@tauri-apps/api/core` invoke() path by stubbing
// the WebView bridge (`window.__TAURI_INTERNALS__`), which is the same surface
// the Rust command layer exposes in a live renderer.

function stubInvoke() {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  let handler: (cmd: string, args: Record<string, unknown>) => Promise<unknown> =
    async () => ({ audio_b64: '', mime: 'audio/mpeg' });
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {
      invoke: vi.fn(async (cmd: string, args: Record<string, unknown>) => {
        calls.push({ cmd, args });
        return handler(cmd, args);
      }),
    },
  });
  const setHandler = (
    h: (cmd: string, args: Record<string, unknown>) => Promise<unknown>,
  ) => {
    handler = h;
  };
  return { calls, setHandler };
}

function stubAudio() {
  const played: { b64: string; mime: string }[] = [];
  const playMock = vi.fn().mockImplementation(function (this: { onended: (() => void) | null }) {
    // Simulate completion of playback so playAudio()'s promise resolves.
    setTimeout(() => this.onended?.(), 0);
    return Promise.resolve();
  });
  const urlMap = new Map<string, { b64: string; mime: string }>();
  vi.stubGlobal('URL', {
    createObjectURL: vi.fn((blob: Blob) => {
      const id = `blob:${played.length}`;
      urlMap.set(id, {
        b64: blob.size ? 'x' : '',
        mime: blob.type,
      });
      return id;
    }),
    revokeObjectURL: vi.fn(),
  });
  vi.stubGlobal('Audio', class {
    onended: (() => void) | null = null;
    onerror: (() => void) | null = null;
    src: string;
    constructor(src: string) {
      this.src = src;
      const u = urlMap.get(src);
      if (u) played.push({ b64: u.b64, mime: u.mime });
    }
    play = playMock;
  });
  return { playMock, played };
}

describe('plainTextFromContent', () => {
  it('extracts styled text and link labels, skipping link hrefs', () => {
    const content = [
      { type: 'text', text: 'Hello ', styles: {} },
      {
        type: 'link',
        href: 'http://example.com',
        content: [{ type: 'text', text: 'world', styles: {} }],
      },
      { type: 'text', text: '!', styles: {} },
    ];
    expect(plainTextFromContent(content)).toBe('Hello world!');
  });

  it('handles string, empty, and undefined content', () => {
    expect(plainTextFromContent('raw string')).toBe('raw string');
    expect(plainTextFromContent([])).toBe('');
    expect(plainTextFromContent(undefined)).toBe('');
    expect(plainTextFromContent(null)).toBe('');
  });

  it('treats a link with a string label as its label', () => {
    expect(plainTextFromContent([
      { type: 'link', href: 'x', content: 'label' },
    ])).toBe('label');
  });
});

describe('textFromBlock', () => {
  it('returns the block plain text', () => {
    const block = {
      id: 'b1',
      content: [{ type: 'text', text: 'block text', styles: {} }],
    };
    expect(textFromBlock(block)).toBe('block text');
  });
});

describe('speakText', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('invokes tts_speak with the text and plays the returned audio', async () => {
    const audioB64 = btoa('fake-mp3-bytes');
    const { calls, setHandler } = stubInvoke();
    setHandler(async (cmd) => {
      expect(cmd).toBe('tts_speak');
      return { audio_b64: audioB64, mime: 'audio/mpeg', byte_len: 14, model: 'tts-1' };
    });
    const { playMock, played } = stubAudio();

    await speakText('Say this');

    expect(calls).toHaveLength(1);
    expect(calls[0].cmd).toBe('tts_speak');
    expect(calls[0].args).toEqual({ text: 'Say this' });
    expect(played).toContainEqual({ b64: 'x', mime: 'audio/mpeg' });
    expect(playMock).toHaveBeenCalledTimes(1);
  });

  it('propagates the endpoint-down error from the backend', async () => {
    const { setHandler } = stubInvoke();
    setHandler(async () => {
      throw new Error('TTS failed: connection refused (http://localhost:18080)');
    });
    stubAudio();

    await expect(speakText('Say this')).rejects.toThrow(
      /TTS failed: connection refused/,
    );
  });
});
