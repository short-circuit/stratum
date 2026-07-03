import appIconUrl from '../../src-tauri/icons/app-icon.svg';

interface StratumIconProps {
  className?: string;
  style?: React.CSSProperties;
}

export default function StratumIcon({ className, style }: StratumIconProps) {
  return (
    <img
      src={appIconUrl}
      alt="Stratum"
      className={className}
      style={{ ...style, display: 'block' }}
      draggable={false}
    />
  );
}
