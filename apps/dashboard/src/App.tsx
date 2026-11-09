import { useQuery } from "@tanstack/react-query";

import { healthQueryOptions } from "./data/health/query";

export default function App() {
  const { data } = useQuery(healthQueryOptions());
  const status = data?.status ?? "offline";

  return (
    <main>
      <h1>Kira Dashboard</h1>
      <p>Server: {status}</p>
    </main>
  );
}
