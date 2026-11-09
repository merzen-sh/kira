import { useHealthQuery } from "./data/health/query";

export default function App() {
  const { data } = useHealthQuery();
  const status = data?.status ?? "offline";

  return (
    <main>
      <h1>Kira Dashboard</h1>
      <p>Server: {status}</p>
    </main>
  );
}
