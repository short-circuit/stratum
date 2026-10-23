//! Shared collapsible settings accordion.
//!
//! Provides a consistent collapsible container for settings groups, controlled
//! or uncontrolled. In controlled mode the caller supplies `expanded` and
//! `onChange`; in uncontrolled mode the component manages its own open state,
//! seeded by `defaultOpen`. The expand icon is a ChevronRight that rotates 90°
//! when open. Callers pass a `summary` — either static content or a function
//! receiving the current open state — following the consistent summary pattern
//! (e.g. a `settings.configure_*` / `settings.hide_*_config` label).

import { useState } from 'react';
import type { ReactNode } from 'react';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';

interface SettingsAccordionProps {
  summary: ReactNode | ((open: boolean) => ReactNode);
  /** Controlled open state; when provided the component is fully controlled. */
  expanded?: boolean;
  /** Always fired on toggle with the next open state. */
  onChange?: (open: boolean) => void;
  /** Uncontrolled initial open state (default false). */
  defaultOpen?: boolean;
  children?: ReactNode;
}

export default function SettingsAccordion({
  summary,
  expanded,
  onChange,
  defaultOpen = false,
  children,
}: SettingsAccordionProps) {
  const [uncontrolledOpen, setUncontrolledOpen] = useState(defaultOpen);
  const isControlled = expanded !== undefined;
  const open = isControlled ? expanded : uncontrolledOpen;

  const handleToggle = () => {
    const next = !open;
    if (!isControlled) setUncontrolledOpen(next);
    onChange?.(next);
  };

  const summaryContent = typeof summary === 'function' ? summary(open) : summary;

  return (
    <Accordion
      disableGutters
      square
      expanded={open}
      onChange={handleToggle}
      sx={{ boxShadow: 0, '&:before': { display: 'none' }, bgcolor: 'transparent' }}
    >
      <AccordionSummary
        expandIcon={
          <ChevronRightIcon
            sx={{ transform: open ? 'rotate(90deg)' : 'none', transition: 'transform 150ms ease-in-out' }}
          />
        }
        sx={{ px: 0, minHeight: 36 }}
      >
        {summaryContent}
      </AccordionSummary>
      <AccordionDetails sx={{ px: 0, pb: 1 }}>{children}</AccordionDetails>
    </Accordion>
  );
}
