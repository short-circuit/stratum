/**
 * Decode base64 audio bytes and play them in an ephemeral <audio> element.
 *
 * Used by the read-aloud surfaces (AI formatting toolbar, Ask Notes panel).
 */
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
