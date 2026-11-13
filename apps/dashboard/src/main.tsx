import "@kira/ui/globals.css";
import "./index.css";

import {
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { authKeys } from "./data/auth/keys";
import { setUnauthorizedHandler } from "./data/fetcher";
import { router } from "./router";

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error) => console.error("API query failed", error),
  }),
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
});

// A 401 means the signed session expired or was cleared: drop back to login.
setUnauthorizedHandler(() => {
  queryClient.setQueryData(authKeys.session(), false);
});

const root = document.getElementById("root");
if (!root) {
  throw new Error("root element missing");
}

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);
