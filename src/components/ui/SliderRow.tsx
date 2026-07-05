import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Slider from '@mui/material/Slider';

interface SliderRowProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  display: string;
}

export default function SliderRow({ label, value, min, max, step, onChange, display }: SliderRowProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
      <Typography variant="caption" sx={{ width: 100, textAlign: 'right', color: 'text.secondary', flexShrink: 0 }}>{label}</Typography>
      <Slider
        size="small"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(_, v) => onChange(v as number)}
        sx={{ flex: 1 }}
      />
      <Typography variant="caption" sx={{ width: 48, textAlign: 'left', color: 'text.disabled', fontFamily: 'monospace', fontSize: '0.7rem' }}>{display}</Typography>
    </Box>
  );
}
