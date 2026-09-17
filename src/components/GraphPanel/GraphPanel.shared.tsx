// Shared logic for GraphPanel — state management, data loading, filtering, navigation.
// Provides the useGraphPanel hook consumed by both desktop and mobile variants.
// The hook implementation lives in ./useGraphPanel (extracted during the E6 sizing-gate refactor).
/* eslint-disable react-refresh/only-export-components */
export { useGraphPanel, type UseGraphPanelReturn } from './useGraphPanel';
