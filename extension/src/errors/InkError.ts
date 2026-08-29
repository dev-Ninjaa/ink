export type InkErrorCategory = "Runtime" | "MCP" | "Configuration" | "Workspace" | "Validation" | "Unknown";

export interface InkErrorShape {
  readonly code: string;
  readonly message: string;
  readonly category: InkErrorCategory;
  readonly details?: unknown;
  readonly timestamp: Date;
}

export class InkError extends Error implements InkErrorShape {
  readonly code: string;
  readonly category: InkErrorCategory;
  readonly details?: unknown;
  readonly timestamp: Date;

  constructor(error: Omit<InkErrorShape, "timestamp"> & { readonly timestamp?: Date }) {
    super(error.message);
    this.name = "InkError";
    this.code = error.code;
    this.category = error.category;
    this.details = error.details;
    this.timestamp = error.timestamp ?? new Date();
  }
}

export function toInkError(error: unknown, fallbackCategory: InkErrorCategory = "Unknown"): InkError {
  if (error instanceof InkError) {
    return error;
  }

  if (error instanceof Error) {
    return new InkError({
      code: `${fallbackCategory.toUpperCase()}_ERROR`,
      message: error.message,
      category: fallbackCategory,
      details: error.stack
    });
  }

  return new InkError({
    code: `${fallbackCategory.toUpperCase()}_ERROR`,
    message: "An unknown Ink error occurred.",
    category: fallbackCategory,
    details: error
  });
}

export function getUserMessage(error: InkError): string {
  switch (error.category) {
    case "Workspace":
      return error.message;
    case "Configuration":
      return `Ink configuration error: ${error.message}`;
    case "Runtime":
      return `Ink runtime error: ${error.message}`;
    case "MCP":
      return `Ink MCP error: ${error.message}`;
    case "Validation":
      return `Ink validation error: ${error.message}`;
    case "Unknown":
      return "Ink encountered an unexpected error. See the INK output channel for details.";
  }
}
