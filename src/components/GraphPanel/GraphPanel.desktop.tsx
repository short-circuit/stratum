import { useEffect, useState } from 'react';
import { useTheme } from '@mui/material/styles';
import Box from '@mui/material/Box';
import { useGraphPanel } from './GraphPanel.shared';
import GraphToolbar from './GraphToolbar';
import GraphSettingsPanel from './GraphSettings';
import GraphCanvas from './GraphCanvas';

export default function GraphPanelDesktop() {
  const muiTheme = useTheme();
  const bgColor = muiTheme.palette.background.default;
  const textColor = muiTheme.palette.text.primary;

  const {
    state: { graphData, loading, error, viewMode, selectedComponent, search,
            graphSettings, saveStatus, graphRef, components, orphans },
    setViewMode, setSelectedComponent, setSearch, loadData,
    handleNodeClick, handleNodeRightClick, updateSetting,
    filteredNodes, filteredEdges, graphDataProp,
  } = useGraphPanel();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [width, setWidth] = useState(800);
  const [height, setHeight] = useState(600);

  // Resize handler — desktop: subtract sidebar (240 px) + toolbar row (48 px) + settings panel height
  useEffect(() => {
    const updateSize = () => {
      const settingsH = settingsOpen ? 160 : 0;
      setWidth(window.innerWidth - 240);
      setHeight(window.innerHeight - 48 - settingsH);
    };
    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, [settingsOpen]);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <GraphToolbar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        loading={loading}
        onRefresh={loadData}
        components={components}
        selectedComponent={selectedComponent}
        onSelectedComponentChange={setSelectedComponent}
        search={search}
        onSearchChange={(v) => {
          setSearch(v);
          if (v) setViewMode('full');
        }}
        settingsOpen={settingsOpen}
        onSettingsToggle={() => setSettingsOpen((o) => !o)}
        nodes={filteredNodes}
        edges={filteredEdges}
        orphans={orphans}
        graphData={graphData}
        saveStatus={saveStatus}
      />

      <GraphSettingsPanel
        settingsOpen={settingsOpen}
        graphSettings={graphSettings}
        updateSetting={updateSetting}
      />

      <GraphCanvas
        graphDataProp={graphDataProp}
        width={width}
        height={height}
        bgColor={bgColor}
        textColor={textColor}
        handleNodeClick={handleNodeClick}
        handleNodeRightClick={handleNodeRightClick}
        loading={loading}
        error={error}
        nodes={filteredNodes}
        graphData={graphData}
        graphSettings={graphSettings}
        graphRef={graphRef}
      />
    </Box>
  );
}
