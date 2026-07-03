import { useState } from 'react';
import Box from '@mui/material/Box';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';

interface Category {
  label: string;
  symbols: { char: string; latex: string }[];
}

const CATEGORIES: Category[] = [
  {
    label: 'Greek',
    symbols: [
      { char: 'α', latex: '\\alpha' },
      { char: 'β', latex: '\\beta' },
      { char: 'γ', latex: '\\gamma' },
      { char: 'δ', latex: '\\delta' },
      { char: 'ε', latex: '\\varepsilon' },
      { char: 'ζ', latex: '\\zeta' },
      { char: 'η', latex: '\\eta' },
      { char: 'θ', latex: '\\theta' },
      { char: 'ι', latex: '\\iota' },
      { char: 'κ', latex: '\\kappa' },
      { char: 'λ', latex: '\\lambda' },
      { char: 'μ', latex: '\\mu' },
      { char: 'ν', latex: '\\nu' },
      { char: 'ξ', latex: '\\xi' },
      { char: 'ο', latex: '\\omicron' },
      { char: 'π', latex: '\\pi' },
      { char: 'ρ', latex: '\\rho' },
      { char: 'σ', latex: '\\sigma' },
      { char: 'τ', latex: '\\tau' },
      { char: 'υ', latex: '\\upsilon' },
      { char: 'φ', latex: '\\phi' },
      { char: 'χ', latex: '\\chi' },
      { char: 'ψ', latex: '\\psi' },
      { char: 'ω', latex: '\\omega' },
    ],
  },
  {
    label: 'Capital Gk',
    symbols: [
      { char: 'Α', latex: '\\Alpha' },
      { char: 'Β', latex: '\\Beta' },
      { char: 'Γ', latex: '\\Gamma' },
      { char: 'Δ', latex: '\\Delta' },
      { char: 'Ε', latex: '\\Epsilon' },
      { char: 'Ζ', latex: '\\Zeta' },
      { char: 'Η', latex: '\\Eta' },
      { char: 'Θ', latex: '\\Theta' },
      { char: 'Ι', latex: '\\Iota' },
      { char: 'Κ', latex: '\\Kappa' },
      { char: 'Λ', latex: '\\Lambda' },
      { char: 'Μ', latex: '\\Mu' },
      { char: 'Ν', latex: '\\Nu' },
      { char: 'Ξ', latex: '\\Xi' },
      { char: 'Ο', latex: '\\Omicron' },
      { char: 'Π', latex: '\\Pi' },
      { char: 'Ρ', latex: '\\Rho' },
      { char: 'Σ', latex: '\\Sigma' },
      { char: 'Τ', latex: '\\Tau' },
      { char: 'Υ', latex: '\\Upsilon' },
      { char: 'Φ', latex: '\\Phi' },
      { char: 'Χ', latex: '\\Chi' },
      { char: 'Ψ', latex: '\\Psi' },
      { char: 'Ω', latex: '\\Omega' },
    ],
  },
  {
    label: 'Operators',
    symbols: [
      { char: '∑', latex: '\\sum' },
      { char: '∏', latex: '\\prod' },
      { char: '∫', latex: '\\int' },
      { char: '∬', latex: '\\iint' },
      { char: '∭', latex: '\\iiint' },
      { char: '∂', latex: '\\partial' },
      { char: '∇', latex: '\\nabla' },
      { char: '∞', latex: '\\infty' },
      { char: '∅', latex: '\\emptyset' },
    ],
  },
  {
    label: 'Relations',
    symbols: [
      { char: '≤', latex: '\\leq' },
      { char: '≥', latex: '\\geq' },
      { char: '≠', latex: '\\neq' },
      { char: '≈', latex: '\\approx' },
      { char: '≡', latex: '\\equiv' },
      { char: '≅', latex: '\\cong' },
      { char: '∼', latex: '\\sim' },
      { char: '⊥', latex: '\\perp' },
      { char: '∣', latex: '\\mid' },
      { char: '∥', latex: '\\parallel' },
    ],
  },
  {
    label: 'Arrows',
    symbols: [
      { char: '→', latex: '\\rightarrow' },
      { char: '←', latex: '\\leftarrow' },
      { char: '↑', latex: '\\uparrow' },
      { char: '↓', latex: '\\downarrow' },
      { char: '⇒', latex: '\\Rightarrow' },
      { char: '⇐', latex: '\\Leftarrow' },
      { char: '⇔', latex: '\\Leftrightarrow' },
      { char: '↔', latex: '\\leftrightarrow' },
      { char: '↦', latex: '\\mapsto' },
      { char: '⟹', latex: '\\implies' },
      { char: '⟸', latex: '\\impliedby' },
      { char: '⟺', latex: '\\iff' },
    ],
  },
  {
    label: 'Sets',
    symbols: [
      { char: '∈', latex: '\\in' },
      { char: '∉', latex: '\\notin' },
      { char: '⊆', latex: '\\subseteq' },
      { char: '⊂', latex: '\\subset' },
      { char: '⊇', latex: '\\supseteq' },
      { char: '⊃', latex: '\\supset' },
      { char: '∪', latex: '\\cup' },
      { char: '∩', latex: '\\cap' },
      { char: '∖', latex: '\\setminus' },
    ],
  },
  {
    label: 'Templates',
    symbols: [
      { char: 'a\n─\nb', latex: '\\frac{}{}' },
      { char: '√', latex: '\\sqrt{}' },
      { char: 'ⁿ√', latex: '\\sqrt[]{}' },
      { char: '∑', latex: '\\sum_{}^{}' },
      { char: '∫', latex: '\\int_{}^{}' },
      { char: 'lim', latex: '\\lim_{}' },
      { char: '(ⁿ\nₖ)', latex: '\\binom{}{}' },
      { char: '⬇', latex: '\\underset{}{}' },
      { char: '‾', latex: '\\overline{}' },
      { char: '＿', latex: '\\underline{}' },
    ],
  },
];

interface Props {
  onInsert: (latex: string) => void;
}

export default function MathSymbolPalette({ onInsert }: Props) {
  const [activeCategory, setActiveCategory] = useState(0);

  return (
    <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
      <Tabs
        value={activeCategory}
        onChange={(_, i) => setActiveCategory(i)}
        variant="scrollable"
        scrollButtons="auto"
        sx={{ minHeight: 36, '& .MuiTab-root': { minHeight: 36, py: 0, fontSize: '0.7rem', textTransform: 'none' } }}
      >
        {CATEGORIES.map(cat => <Tab key={cat.label} label={cat.label} />)}
      </Tabs>
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.25, p: 0.75 }}>
        <ToggleButtonGroup size="small">
          {CATEGORIES[activeCategory].symbols.map(sym => (
            <ToggleButton
              key={sym.latex}
              value={sym.latex}
              selected={false}
              onClick={() => onInsert(sym.latex)}
              title={sym.latex}
              sx={{
                p: 0.25, minWidth: 28, height: 28, borderRadius: '4px!important',
                fontSize: CATEGORIES[activeCategory].label === 'Templates' ? '0.65rem' : '0.8rem',
                whiteSpace: CATEGORIES[activeCategory].label === 'Templates' ? 'pre-line' : undefined,
                lineHeight: CATEGORIES[activeCategory].label === 'Templates' ? 1.1 : undefined,
              }}
            >
              {sym.char}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>
      </Box>
    </Box>
  );
}
