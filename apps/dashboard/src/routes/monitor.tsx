import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@kira/ui/components/select";
import { createFileRoute } from "@tanstack/react-router";
import { Cpu, HardDrive, MemoryStick, type LucideIcon } from "lucide-react";
import { useState } from "react";

import { UsageChart, type UsagePoint } from "@/components/usage-chart";
import {
  type MonitorRange,
  type MonitorSample,
  useMonitorQuery,
} from "@/data/monitor/query";

export const Route = createFileRoute("/monitor")({
  component: MonitorPage,
});

const RANGE_ITEMS: Record<MonitorRange, string> = {
  "1h": "Last hour",
  "3h": "Last 3 hours",
  "24h": "Last 24 hours",
};

const RANGES: MonitorRange[] = ["1h", "3h", "24h"];

function percentOf(used: number, total: number) {
  return total > 0 ? (used / total) * 100 : 0;
}

function formatPercent(value: number | undefined) {
  return value === undefined ? "—" : `${value.toFixed(1)}%`;
}

function formatBytes(value: number | undefined) {
  if (value === undefined) {
    return "—";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let scaled = value;
  let unit = 0;

  while (scaled >= 1024 && unit < units.length - 1) {
    scaled /= 1024;
    unit += 1;
  }

  return `${scaled.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function formatUsage(used: number | undefined, total: number | undefined) {
  if (used === undefined || total === undefined) {
    return "Waiting for a sample";
  }

  return `${formatBytes(used)} of ${formatBytes(total)}`;
}

function series(
  history: MonitorSample[],
  value: (sample: MonitorSample) => number,
): UsagePoint[] {
  return history.map((sample) => ({
    recorded_at: sample.recorded_at,
    value: value(sample),
  }));
}

interface UsageCardProps {
  label: string;
  icon: LucideIcon;
  color: string;
  percent: number | undefined;
  detail: string;
  points: UsagePoint[];
}

function UsageCard({
  label,
  icon: Icon,
  color,
  percent,
  detail,
  points,
}: UsageCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardDescription className="flex items-center gap-2">
          <Icon className="size-4" />
          {label}
        </CardDescription>
        <CardTitle className="text-2xl font-semibold tabular-nums">
          {formatPercent(percent)}
        </CardTitle>
        <p className="text-xs text-muted-foreground">{detail}</p>
      </CardHeader>
      <CardContent>
        <UsageChart label={label} color={color} points={points} />
      </CardContent>
    </Card>
  );
}

function MonitorPage() {
  const [range, setRange] = useState<MonitorRange>("1h");
  const monitor = useMonitorQuery(range);
  const current = monitor.data?.current;
  const history = monitor.data?.history ?? [];

  return (
    <main className="flex flex-1 flex-col gap-4 p-6 md:p-8">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">System monitor</h1>
          <p className="text-sm text-muted-foreground">
            Host CPU, memory, and disk usage.
          </p>
        </div>
        <Select
          value={range}
          onValueChange={(value) => {
            if (value) {
              setRange(value as MonitorRange);
            }
          }}
          items={RANGE_ITEMS}
        >
          <SelectTrigger
            aria-label="History range"
            className="w-full sm:w-40"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {RANGES.map((value) => (
              <SelectItem key={value} value={value}>
                {RANGE_ITEMS[value]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </header>

      {monitor.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {monitor.error.message}
        </p>
      ) : null}

      <section
        aria-label="Resource usage"
        className="grid gap-3 md:grid-cols-3"
      >
        <UsageCard
          label="CPU"
          icon={Cpu}
          color="var(--chart-1)"
          percent={current?.cpu_percent}
          detail="Host-wide utilization"
          points={series(history, (sample) => sample.cpu_percent)}
        />
        <UsageCard
          label="Memory"
          icon={MemoryStick}
          color="var(--chart-2)"
          percent={
            current ? percentOf(current.memory_used, current.memory_total) : undefined
          }
          detail={formatUsage(current?.memory_used, current?.memory_total)}
          points={series(history, (sample) =>
            percentOf(sample.memory_used, sample.memory_total),
          )}
        />
        <UsageCard
          label="Disk"
          icon={HardDrive}
          color="var(--chart-3)"
          percent={
            current ? percentOf(current.disk_used, current.disk_total) : undefined
          }
          detail={formatUsage(current?.disk_used, current?.disk_total)}
          points={series(history, (sample) =>
            percentOf(sample.disk_used, sample.disk_total),
          )}
        />
      </section>
    </main>
  );
}
