import { createFileRoute } from "@tanstack/react-router";

import { useHealthQuery } from "@/data/health/query";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { data } = useHealthQuery();
  const status = data?.status ?? "offline";

  return (
    <main>
      <h1>Kira Dashboard</h1>
      <p>Server: {status}</p>
    </main>
  );
}
