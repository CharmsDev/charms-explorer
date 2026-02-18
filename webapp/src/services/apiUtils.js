"use client";

// Utility functions for API operations

// Conditional logger - only logs in development
const isDev = process.env.NODE_ENV === "development";
export const logger = {
  error: (context, error) => isDev && console.error(`[${context}]`, error),
  warn: (context, msg) => isDev && console.warn(`[${context}]`, msg),
  info: (context, msg) => isDev && console.info(`[${context}]`, msg),
};

// Safely extracts a property from an object with nested fallbacks
export const getNestedProperty = (obj, path, defaultValue = undefined) => {
  if (!obj || !path) return defaultValue;

  const keys = path.split(".");
  let current = obj;

  for (const key of keys) {
    if (
      current === null ||
      current === undefined ||
      typeof current !== "object"
    ) {
      return defaultValue;
    }

    // Handle array access with [index] notation
    if (key.includes("[") && key.includes("]")) {
      const arrayKey = key.substring(0, key.indexOf("["));
      const indexStr = key.substring(key.indexOf("[") + 1, key.indexOf("]"));
      const index = parseInt(indexStr, 10);

      if (
        current[arrayKey] &&
        Array.isArray(current[arrayKey]) &&
        !isNaN(index)
      ) {
        current = current[arrayKey][index];
      } else {
        return defaultValue;
      }
    } else {
      current = current[key];
    }
  }

  return current !== undefined ? current : defaultValue;
};

// Handles API errors consistently
export const handleApiError = (error, context) => {
  logger.error(context, error);
  const enhancedError = new Error(`Failed to ${context}`);
  enhancedError.originalError = error;
  return enhancedError;
};
