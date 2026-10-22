//! Shared collapsible settings accordion.
//!
//! Mirrors the previous mobile AI accordion styling (frameless, transparent,
//! no divider line) so any collapsible settings group on mobile renders
//! identically to the historical behavior.

import type { ReactNode } from 'react';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';

interface SettingsAccordionProps {
  summary: ReactNode;
  expanded: boolean;
  onChange: (open: boolean) => void;
  children?: ReactNode;
}

export default function SettingsAccordion({ summary, expanded, onChange, children }: SettingsAccordionProps) {
  return (
    <Accordion
      disableGutters
      square
      expanded={expanded}
      onChange={() => onChange(!expanded)}
      sx={{ boxShadow: 0, '&:before': { display: 'none' }, bgcolor: 'transparent' }}
    >
      <AccordionSummary expandIcon={<ExpandMoreIcon />} sx={{ px: 0, minHeight: 36 }}>
        {summary}
      </AccordionSummary>
      <AccordionDetails sx={{ px: 0, pb: 1 }}>{children}</AccordionDetails>
    </Accordion>
  );
}
