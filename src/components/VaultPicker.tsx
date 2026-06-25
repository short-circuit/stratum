import { useStore } from '../stores/appStore';

export default function VaultPicker() {
  const { pickVaultDirectory, error } = useStore();

  return (
    <div className="flex items-center justify-center h-screen w-screen bg-white dark:bg-[var(--secondary-900)]">
      <div className="max-w-md w-full mx-4 p-8 rounded-xl border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] shadow-lg">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-bold text-[var(--secondary-800)] dark:text-[var(--secondary-100)] mb-2">
            Welcome to Stratum
          </h1>
          <p className="text-sm text-[var(--secondary-500)] dark:text-[var(--secondary-400)]">
            Select or create a vault to get started.
            Your notes are stored as plain Markdown files.
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 text-sm rounded bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 border border-red-200 dark:border-red-800">
            {error}
          </div>
        )}

        <div className="space-y-3">
          <button
            onClick={pickVaultDirectory}
            className="w-full flex items-center justify-center gap-3 px-4 py-3 rounded-lg bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)] disabled:opacity-50 transition-colors"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
            </svg>
            Choose Vault Folder
          </button>
          <p className="text-xs text-center text-[var(--secondary-400)]">
            Opens a folder picker to select or create a vault directory.
            A <code className="px-1 py-0.5 rounded bg-[var(--secondary-100)] dark:bg-[var(--secondary-800)]">.pkm</code> folder will be created inside.
          </p>
        </div>
      </div>
    </div>
  );
}
