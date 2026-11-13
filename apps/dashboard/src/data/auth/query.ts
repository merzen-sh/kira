import { useQuery } from "@tanstack/react-query";

import { GET } from "@/data/fetcher";
import { authKeys } from "./keys";

/** The session cookie is HttpOnly, so the API is the only source of truth. */
export async function checkSession(): Promise<boolean> {
  const { response } = await GET("/auth/session");
  return response.ok;
}

export const useSessionQuery = () =>
  useQuery({
    queryKey: authKeys.session(),
    queryFn: checkSession,
    retry: false,
    staleTime: 60_000,
  });
