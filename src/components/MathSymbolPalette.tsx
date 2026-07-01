import { useState } from 'react';

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
    <div className="math-symbol-palette border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] pb-1">
      <div className="flex gap-1 overflow-x-auto px-1 py-1 text-[10px]">
        {CATEGORIES.map((cat, i) => (
          <button
            key={cat.label}
            onClick={() => setActiveCategory(i)}
            className={`px-2 py-0.5 rounded whitespace-nowrap transition-colors ${
              i === activeCategory
                ? 'bg-[var(--primary-200)] dark:bg-[var(--primary-800)] text-[var(--primary-700)] dark:text-[var(--primary-300)] font-medium'
                : 'text-[var(--secondary-500)] hover:text-[var(--secondary-700)] dark:hover:text-[var(--secondary-300)]'
            }`}
          >
            {cat.label}
          </button>
        ))}
      </div>
      <div className="flex flex-wrap gap-0.5 px-1 py-1">
        {CATEGORIES[activeCategory].symbols.map((sym) => (
          <button
            key={sym.latex}
            onClick={() => onInsert(sym.latex)}
            title={sym.latex}
            className={`flex items-center justify-center text-sm rounded hover:bg-[var(--primary-100)] dark:hover:bg-[var(--primary-900)] transition-colors border border-transparent hover:border-[var(--primary-300)] dark:hover:border-[var(--primary-700)] ${CATEGORIES[activeCategory].label === 'Templates' ? 'w-10 h-9 whitespace-pre-line leading-tight text-xs' : 'w-7 h-7'}`}
          >
            {sym.char}
          </button>
        ))}
      </div>
    </div>
  );
}
