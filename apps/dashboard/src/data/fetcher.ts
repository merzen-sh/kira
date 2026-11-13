import type { paths } from "@kira/api-types";
import createClient from "openapi-fetch";

import type { ResourceError } from "./types";

const DEFAULT_API_PORT = "8080";

function defaultBaseUrl(): string {
  if (typeof window === "undefined") {
    return `http://127.0.0.1:${DEFAULT_API_PORT}`;
  }

  return `${window.location.protocol}//${window.location.hostname}:${DEFAULT_API_PORT}`;
}

/** Base URL of the Kira API. Override with `VITE_API_BASE_URL` when the API
 * lives on a different host or port than the dashboard. */
export const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? defaultBaseUrl();

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

export const client = createClient<paths>({
  baseUrl: API_BASE_URL,
  credentials: "include",
  headers: { Accept: "application/json" },
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

/** Convert any thrown value into the dashboard's shared error shape. */
export function toResourceError(error: unknown): ResourceError {
  if (error instanceof ApiError) {
    const payload = error.payload;

    if (typeof payload === "object" && payload !== null) {
      if ("message" in payload && typeof payload.message === "string") {
        return { message: payload.message };
      }

      if ("error" in payload && typeof payload.error === "string") {
        return { message: payload.error };
      }
    }

    return { message: error.message };
  }

  if (error instanceof Error) {
    return { message: error.message };
  }

  return { message: "unknown API error" };
}
