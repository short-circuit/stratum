//! Shared settings tab bar.
//!
//! Renders a compact MUI Tabs bar (minHeight 40, no text transform) for the
//! desktop SettingsPage so the tab strip is a single reusable primitive instead
//! of an inline copy. Labels are provided already-translated by the caller.

import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';

interface SettingsTabProps {
  value: string;
  onChange: (value: string) => void;
  tabs: { value: string; label: string }[];
}

export default function SettingsTab({ value, onChange, tabs }: SettingsTabProps) {
  return (
    <Tabs value={value} onChange={(_, v) => onChange(v)} sx={{ minHeight: 40 }}>
      {tabs.map(tab => (
        <Tab
          key={tab.value}
          label={tab.label}
          value={tab.value}
          sx={{ minHeight: 40, textTransform: 'none' }}
        />
      ))}
    </Tabs>
  );
}
