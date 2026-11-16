import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import type { LucideIcon } from "lucide-react";

interface StatCardProps {
  label: string;
  value: number | undefined;
  icon: LucideIcon;
  pending?: boolean;
  error?: boolean;
}

export function StatCard({
  label,
  value,
  icon: Icon,
  pending = false,
  error = false,
}: StatCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardDescription className="flex items-center gap-2">
          <Icon className="size-4" />
          {label}
        </CardDescription>
        <CardTitle className="text-2xl font-semibold tabular-nums">
          {pending ? "…" : error ? "—" : (value ?? 0)}
        </CardTitle>
      </CardHeader>
    </Card>
  );
}
