import { ttsSpeak } from './commands/features';

/**
 * Recursively extract plain text from a BlockNote block's inline content.
 *
 * Handles the BlockNote content shapes: styled text `{type:'text', text}` and
 * links `{type:'link', href, content: StyledText[]}` (links contribute their
 * visible label, not the href).
 */
export function plainTextFromContent(
  content: unknown,
): string {
  if (typeof content === 'string') return content;
  if (!Array.isArray(content)) return '';
  const parts: string[] = [];
  for (const item of content) {
    if (!item || typeof item !== 'object') continue;
    const c = item as Record<string, unknown>;
    if (c.type === 'link') {
      parts.push(plainTextFromContent(c.content));
    } else if (typeof c.text === 'string') {
      parts.push(c.text);
    }
  }
  return parts.join('');
}

/**
 * Extract the readable text from a single editor block.
 */
export function textFromBlock(block: { id: string; content?: unknown }): string {
  return plainTextFromContent(block.content);
}

/**
 * Synthesize `text` via the configured TTS endpoint and play it back.
 *
 * Rejects with the backend's error message when the endpoint is down or
 * unconfigured so callers can surface a user-facing error.
 */
export async function speakText(text: string): Promise<void> {
  const res = await ttsSpeak(text);
  await playAudio(res.audio_b64, res.mime);
}

export async function playAudio(audioB64: string, mime: string): Promise<void> {
  const byteCharacters = atob(audioB64);
  const byteNumbers = new Array(byteCharacters.length);
  for (let i = 0; i < byteCharacters.length; i += 1) {
    byteNumbers[i] = byteCharacters.charCodeAt(i);
  }
  const byteArray = new Uint8Array(byteNumbers);
  const blob = new Blob([byteArray], { type: mime });
  const url = URL.createObjectURL(blob);
  try {
    await new Promise<void>((resolve, reject) => {
      const audio = new Audio(url);
      audio.onended = () => {
        URL.revokeObjectURL(url);
        resolve();
      };
      audio.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error('Failed to play audio'));
      };
      void audio.play().catch(reject);
    });
  } catch (e) {
    URL.revokeObjectURL(url);
    throw e;
  }
}
