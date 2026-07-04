// Module-level store for the latest library JSON.
// Persists across component mount/unmount cycles so the app-level
// close handler can always access the most recent library data.

let latestJson: string | null = null;

export function setLatestLibraryJson(json: string | null) {
  latestJson = json;
}

export function getLatestLibraryJson(): string | null {
  return latestJson;
}
