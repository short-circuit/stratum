import iconRaw from '../../src-tauri/icons/app-icon.svg?raw';

// Strip hardcoded fill so icon inherits currentColor
const iconSvg = iconRaw.replace(/style="[^"]*"/, '').replace('<svg', '<svg fill="currentColor"');

interface StratumIconProps {
  className?: string;
  style?: React.CSSProperties;
}

export default function StratumIcon({ className, style }: StratumIconProps) {
  return (
    <span
      className={className}
      style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', ...style }}
      dangerouslySetInnerHTML={{ __html: iconSvg }}
    />
  );
}
