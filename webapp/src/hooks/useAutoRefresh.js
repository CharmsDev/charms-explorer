'use client';

import { useEffect, useRef, useCallback } from 'react';

/**
 * Auto-refresh hook for polling API data at a fixed interval.
 *
 * Designed as a standalone module so it can be replaced with WebSocket-based
 * real-time updates in the future without changing consumer code.
 *
 * - The timer is only active while the component is mounted (i.e. the user
 *   is on the page). It is automatically cleaned up on unmount.
 * - The fetch callback runs silently in the background — it never sets a
 *   loading state so the UI doesn't flash.
 * - If a fetch is still in-flight when the next tick fires, it is skipped.
 *
 * @param {Function} fetchFn  - Async function that fetches fresh data.
 *                               Receives no arguments; use closures for params.
 * @param {object}   options
 * @param {number}   options.intervalMs - Polling interval in ms (default 10 000).
 * @param {boolean}  options.enabled    - Set to false to pause polling.
 */
export function useAutoRefresh(fetchFn, { intervalMs = 10_000, enabled = true } = {}) {
  const fetchRef = useRef(fetchFn);
  const inFlightRef = useRef(false);

  // Always keep the latest fetchFn without restarting the timer
  useEffect(() => {
    fetchRef.current = fetchFn;
  }, [fetchFn]);

  const tick = useCallback(async () => {
    if (inFlightRef.current) return; // skip if previous call still running
    inFlightRef.current = true;
    try {
      await fetchRef.current();
    } catch {
      // Silently ignore — the page already shows data from the last good fetch
    } finally {
      inFlightRef.current = false;
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;

    const id = setInterval(tick, intervalMs);
    return () => clearInterval(id);
  }, [tick, intervalMs, enabled]);
}
