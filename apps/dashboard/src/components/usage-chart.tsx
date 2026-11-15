import { useId } from "react";

export interface UsagePoint {
  recorded_at: number;
  value: number;
}

interface UsageChartProps {
  label: string;
  color: string;
  points: UsagePoint[];
  emptyLabel?: string;
}

const WIDTH = 600;
const HEIGHT = 140;
const PADDING = 6;
const TICKS = [25, 50, 75];

function timeLabel(recordedAt: number) {
  return new Date(recordedAt * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function UsageChart({
  label,
  color,
  points,
  emptyLabel = "Collecting data…",
}: UsageChartProps) {
  const gradientId = useId().replaceAll(":", "");

  if (points.length < 2) {
    return (
      <div className="flex h-32 items-center justify-center text-xs text-muted-foreground">
        {emptyLabel}
      </div>
    );
  }

  const first = points[0].recorded_at;
  const span = points[points.length - 1].recorded_at - first;
  const x = (recordedAt: number, index: number) =>
    span > 0
      ? ((recordedAt - first) / span) * WIDTH
      : (index / (points.length - 1)) * WIDTH;
  const y = (value: number) =>
    HEIGHT -
    PADDING -
    (Math.min(Math.max(value, 0), 100) / 100) * (HEIGHT - PADDING * 2);
  const line = points
    .map(
      (point, index) =>
        `${index === 0 ? "M" : "L"}${x(point.recorded_at, index).toFixed(1)},${y(point.value).toFixed(1)}`,
    )
    .join(" ");
  const area = `${line} L${WIDTH},${HEIGHT} L0,${HEIGHT} Z`;

  return (
    <div className="flex flex-col gap-1">
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={`${label} usage history`}
        className="h-32 w-full"
      >
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity={0.3} />
            <stop offset="100%" stopColor={color} stopOpacity={0} />
          </linearGradient>
        </defs>
        {TICKS.map((tick) => (
          <line
            key={tick}
            x1={0}
            x2={WIDTH}
            y1={y(tick)}
            y2={y(tick)}
            stroke="currentColor"
            strokeOpacity={0.1}
            vectorEffect="non-scaling-stroke"
          />
        ))}
        <path d={area} fill={`url(#${gradientId})`} />
        <path
          d={line}
          fill="none"
          stroke={color}
          strokeWidth={2}
          strokeLinejoin="round"
          strokeLinecap="round"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="flex justify-between text-[10px] text-muted-foreground tabular-nums">
        <span>{timeLabel(points[0].recorded_at)}</span>
        <span>{timeLabel(points[points.length - 1].recorded_at)}</span>
      </div>
    </div>
  );
}
