import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";

interface StatCardProps {
  label: string;
  value: number | string;
}

export function StatCard({ label, value }: StatCardProps) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-2xl font-semibold tabular-nums">
          {value}
        </CardTitle>
      </CardHeader>
    </Card>
  );
}
