import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";

interface StatCardProps {
  label: string;
  value: number | undefined;
  pending?: boolean;
  error?: boolean;
}

export function StatCard({
  label,
  value,
  pending = false,
  error = false,
}: StatCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-2xl font-semibold tabular-nums">
          {pending ? "…" : error ? "—" : (value ?? 0)}
        </CardTitle>
      </CardHeader>
    </Card>
  );
}
