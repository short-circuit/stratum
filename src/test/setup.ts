import '@testing-library/jest-dom';
import { initI18n } from '../i18n';

// Render en values (never raw keys) in tests; failures must not break them.
await initI18n('en').catch(() => {});
