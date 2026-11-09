import type { paths } from "@kira/api-types";
import createClient from "openapi-fetch";

const DEFAULT_BASE_URL = "/api";

export interface ApiErrorOptions {
  status?: number;
  payload?: unknown;
  cause?: unknown;
}

/** Normalized error for every dashboard API request. */
export class ApiError extends Error {
  readonly status: number | undefined;
  readonly payload: unknown;

  constructor(message: string, options: ApiErrorOptions = {}) {
    super(message, { cause: options.cause });
    this.name = "ApiError";
    this.status = options.status;
    this.payload = options.payload;
  }
}

export interface ApiResponse<T> {
  data?: T;
  error?: unknown;
  response: Response;
}

let authToken: string | null = null;

/**
 * Set the bearer token used by subsequent requests. Pass `null` to clear it.
 * For cookie-based auth, the client already sends credentials.
 */
export function setAuthToken(token: string | null): void {
  authToken = token;
}

/**
 * The single API client for the dashboard.
 *
 * Add cross-cutting request behavior with `client.use(...)` instead of
 * constructing another client.
 */
export const client = createClient<paths>({
  baseUrl: import.meta.env.VITE_API_BASE_URL ?? DEFAULT_BASE_URL,
  credentials: "include",
  headers: { Accept: "application/json" },
});

client.use({
  onRequest({ request }) {
    if (authToken) {
      request.headers.set("Authorization", `Bearer ${authToken}`);
    }
    return request;
  },
});

export const { GET, POST, PUT, PATCH, DELETE } = client;

/**
 * Unwrap an `openapi-fetch` response, normalizing network, HTTP, and empty-body
 * failures into `ApiError`.
 */
export async function unwrap<T>(request: Promise<ApiResponse<T>>): Promise<T> {
  let result: ApiResponse<T>;

  try {
    result = await request;
  } catch (cause) {
    throw new ApiError("network request failed", { cause });
  }

  if (result.error !== undefined) {
    throw new ApiError(
      `API request failed with status ${result.response.status}`,
      {
        status: result.response.status,
        payload: result.error,
      },
    );
  }

  if (!result.response.ok) {
    throw new ApiError(
      `API request failed with status ${result.response.status}`,
      {
        status: result.response.status,
        payload: result.response.statusText,
      },
    );
  }

  if (result.data === undefined) {
    throw new ApiError("API response contained no data", {
      status: result.response.status,
    });
  }

  return result.data;
}
