import { useEffect, useState } from "react";
import type { components } from "@kira/api-types";

type Health = components["schemas"]["HealthResponse"];

const HEALTH_URL = "/api/health";

export default function App() {
  const [status, setStatus] = useState<string>("offline");

  useEffect(() => {
    const controller = new AbortController();

    fetch(HEALTH_URL, { signal: controller.signal })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`health request failed: ${response.status}`);
        }
        return response.json() as Promise<Health>;
      })
      .then((health) => {
        setStatus(health.status ?? "offline");
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
        console.error(error);
        setStatus("offline");
      });

    return () => controller.abort();
  }, []);

  return (
    <main>
      <h1>Kira Dashboard</h1>
      <p>Server: {status}</p>
    </main>
  );
}
