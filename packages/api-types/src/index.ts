import type {
  components as GeneratedComponents,
  paths as GeneratedPaths,
} from "./schema";

export type { $defs, operations, webhooks } from "./schema";

type GeneratedSchemas = GeneratedComponents["schemas"];

export interface paths extends GeneratedPaths {}

export interface components extends GeneratedComponents {
  schemas: GeneratedSchemas;
}
