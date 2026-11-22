import React from "react";
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RealtimeProvider,
  patchCorridorInInfiniteData,
  patchCorridorInPaginatedData,
} from "@/contexts/RealtimeContext";
import {
  computeJitteredBackoffDelay,
  getRealtimeClient,
  resetRealtimeClient,
} from "@/lib/websocket";
import { useWebSocket } from "@/hooks/useWebSocket";
import { useRealtimeCorridors } from "@/hooks/useRealtimeCorridors";
import { useRealtimeAnchors } from "@/hooks/useRealtimeAnchors";
import { useRealtimeCollaboration } from "@/hooks/useRealtimeCollaboration";
import { queryKeys } from "@/lib/react-query/keys";
import type { CorridorMetrics } from "@/lib/api/corridors";
import type { PaginatedResponse } from "@/lib/api/pagination";

class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  readyState = MockWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  sent: string[] = [];

  constructor(public url: string) {
    MockWebSocket.instances.push(this);
    queueMicrotask(() => {
      if (this.readyState === MockWebSocket.CLOSED) return;
      this.readyState = MockWebSocket.OPEN;
      this.onopen?.();
    });
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({ code: 1000, reason: "" } as CloseEvent);
  }

  emitServerMessage(payload: Record<string, unknown>) {
    this.onmessage?.({
      data: JSON.stringify(payload),
    } as MessageEvent);
  }
}

function createWrapper(queryClient: QueryClient, url = "ws://localhost:8080/ws") {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <RealtimeProvider url={url}>{children}</RealtimeProvider>
      </QueryClientProvider>
    );
  };
}

function makeCorridor(id: string, successRate = 95.0): CorridorMetrics {
  return {
    id,
    source_asset: "USDC",
    destination_asset: "NGN",
    success_rate: successRate,
    total_attempts: 100,
    successful_payments: 95,
    failed_payments: 5,
    average_latency_ms: 420,
    median_latency_ms: 350,
    p95_latency_ms: 800,
    p99_latency_ms: 1200,
    liquidity_depth_usd: 500_000,
    liquidity_volume_24h_usd: 50_000,
    liquidity_trend: "stable",
    average_slippage_bps: 12,
    health_score: 90,
    last_updated: "2026-01-01T00:00:00Z",
  };
}

describe("Single shared RealtimeClient (#2834)", () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    resetRealtimeClient();
    vi.stubGlobal("WebSocket", MockWebSocket);
  });

  afterEach(() => {
    resetRealtimeClient();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("5 components subscribing simultaneously share exactly 1 WebSocket", async () => {
    const queryClient = new QueryClient();

    function SubscriberA() {
      useWebSocket(undefined, { channels: ["notifications"] });
      return null;
    }
    function SubscriberB() {
      useRealtimeCorridors({ corridorKeys: ["USDC->NGN"] });
      return null;
    }
    function SubscriberC() {
      useRealtimeCorridors({ corridorKeys: ["USDC->NGN", "USDC->PHP"] });
      return null;
    }
    function SubscriberD() {
      useRealtimeAnchors({ anchorIds: ["anchor-1"] });
      return null;
    }
    function SubscriberE() {
      useRealtimeCollaboration({
        sessionId: "sess-1",
        userId: "u-1",
        userName: "Alice",
      });
      return null;
    }

    const { unmount } = render(
      <QueryClientProvider client={queryClient}>
        <RealtimeProvider url="ws://localhost:8080/ws">
          <SubscriberA />
          <SubscriberB />
          <SubscriberC />
          <SubscriberD />
          <SubscriberE />
        </RealtimeProvider>
      </QueryClientProvider>,
    );

    await waitFor(() =>
      expect(MockWebSocket.instances[0]?.readyState).toBe(MockWebSocket.OPEN),
    );

    // Exactly ONE socket opened for all 5 subscribing components.
    expect(MockWebSocket.instances).toHaveLength(1);

    const client = getRealtimeClient({ url: "ws://localhost:8080/ws" });
    expect(client.getConsumerCount()).toBe(5);
    // Ref count for shared corridor channel is 2 (SubscriberB + SubscriberC).
    expect(client.getChannelRefCount("corridor:USDC->NGN")).toBe(2);
    expect(client.getChannelRefCount("corridor:USDC->PHP")).toBe(1);
    expect(client.getChannelRefCount("anchor:anchor-1")).toBe(1);
    expect(client.getChannelRefCount("collaboration:sess-1")).toBe(1);
    expect(client.getChannelRefCount("notifications")).toBe(1);

    // Unmounting all 5 components releases channels and closes the single socket.
    unmount();
    expect(client.getConsumerCount()).toBe(0);
    expect(client.getActiveChannels()).toEqual([]);
  });

  it("mock server disconnect triggers exactly 1 reconnect across 5 active subscribers", async () => {
    const queryClient = new QueryClient();
    const client = getRealtimeClient({
      url: "ws://localhost:8080/ws",
      reconnectInterval: 5,
      random: () => 0.5,
    });

    function Sub({ channel }: { channel: string }) {
      useWebSocket(undefined, { channels: [channel] });
      return null;
    }

    render(
      <QueryClientProvider client={queryClient}>
        <RealtimeProvider client={client}>
          <Sub channel="ch-1" />
          <Sub channel="ch-2" />
          <Sub channel="ch-3" />
          <Sub channel="ch-4" />
          <Sub channel="ch-5" />
        </RealtimeProvider>
      </QueryClientProvider>,
    );

    await waitFor(() => expect(MockWebSocket.instances).toHaveLength(1));
    await waitFor(() =>
      expect(MockWebSocket.instances[0].readyState).toBe(MockWebSocket.OPEN),
    );

    // Simulate server disconnect
    act(() => {
      MockWebSocket.instances[0].close();
    });

    // Wait for reconnect — must produce exactly ONE new socket (total = 2), NOT 5!
    await waitFor(() => expect(MockWebSocket.instances).toHaveLength(2));
    await waitFor(() =>
      expect(MockWebSocket.instances[1].readyState).toBe(MockWebSocket.OPEN),
    );

    // Give a brief window to prove no duplicate reconnect sockets spawn.
    await new Promise((r) => setTimeout(r, 30));
    expect(MockWebSocket.instances).toHaveLength(2);

    // The single reconnected socket restores all 5 channels in one subscribe frame.
    const subscribeFrames = MockWebSocket.instances[1].sent
      .map((s) => JSON.parse(s))
      .filter((m) => m.type === "subscribe");
    expect(subscribeFrames).toHaveLength(1);
    expect(subscribeFrames[0].channels).toEqual(
      expect.arrayContaining(["ch-1", "ch-2", "ch-3", "ch-4", "ch-5"]),
    );
  });

  it("ref-counts shared channel subscriptions and unsubscribes only when last holder unmounts", async () => {
    const queryClient = new QueryClient();
    const wrapper = createWrapper(queryClient);

    const hookA = renderHook(
      () => useWebSocket(undefined, { channels: ["corridor:USDC->KES"] }),
      { wrapper },
    );
    const hookB = renderHook(
      () => useWebSocket(undefined, { channels: ["corridor:USDC->KES"] }),
      { wrapper },
    );

    await waitFor(() => expect(hookA.result.current.isConnected).toBe(true));

    const ws = MockWebSocket.instances[0];
    const subCount = ws.sent
      .map((s) => JSON.parse(s))
      .filter(
        (m) =>
          m.type === "subscribe" && m.channels.includes("corridor:USDC->KES"),
      ).length;
    // Wire subscribe sent ONCE even though two hooks acquired the same channel.
    expect(subCount).toBe(1);

    // Unmount first holder -> ref count drops 2 -> 1 -> NO unsubscribe frame sent.
    hookA.unmount();
    const unsubsAfterFirst = ws.sent
      .map((s) => JSON.parse(s))
      .filter((m) => m.type === "unsubscribe");
    expect(unsubsAfterFirst).toHaveLength(0);

    // Unmount second holder -> ref count drops 1 -> 0 -> wire unsubscribe sent.
    hookB.unmount();
    const unsubsAfterSecond = ws.sent
      .map((s) => JSON.parse(s))
      .filter(
        (m) =>
          m.type === "unsubscribe" && m.channels.includes("corridor:USDC->KES"),
      );
    expect(unsubsAfterSecond).toHaveLength(1);
  });

  it("pauses reconnection when tab is hidden and resumes on visibility restore", async () => {
    const queryClient = new QueryClient();
    const client = getRealtimeClient({
      url: "ws://localhost:8080/ws",
      reconnectInterval: 10,
      random: () => 0.5,
    });

    const visibilitySpy = vi
      .spyOn(document, "visibilityState", "get")
      .mockReturnValue("visible");

    renderHook(() => useWebSocket(), {
      wrapper: ({ children }) => (
        <QueryClientProvider client={queryClient}>
          <RealtimeProvider client={client}>{children}</RealtimeProvider>
        </QueryClientProvider>
      ),
    });

    await waitFor(() => expect(MockWebSocket.instances).toHaveLength(1));
    await waitFor(() =>
      expect(MockWebSocket.instances[0].readyState).toBe(MockWebSocket.OPEN),
    );

    // Hide the tab BEFORE server disconnects.
    visibilitySpy.mockReturnValue("hidden");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    act(() => {
      MockWebSocket.instances[0].close();
    });

    // While hidden, no reconnect socket should be created.
    await new Promise((r) => setTimeout(r, 35));
    expect(MockWebSocket.instances).toHaveLength(1);

    // Returning tab to visible resumes reconnect and opens exactly 1 new socket.
    visibilitySpy.mockReturnValue("visible");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    await waitFor(() => expect(MockWebSocket.instances).toHaveLength(2));
    visibilitySpy.mockRestore();
  });

  it("computes bounded jittered exponential backoff delays", () => {
    const low = computeJitteredBackoffDelay(1, 1000, 30_000, 2, () => 0);
    const high = computeJitteredBackoffDelay(1, 1000, 30_000, 2, () => 1);
    expect(low).toBe(500);
    expect(high).toBe(1000);

    const attempt4Cap = computeJitteredBackoffDelay(10, 1000, 30_000, 2, () => 1);
    expect(attempt4Cap).toBe(30_000);
  });

  it("patches React Query corridor list & detail caches in place without touching unrelated rows", async () => {
    const queryClient = new QueryClient();
    const rowA = makeCorridor("USDC->NGN", 94.0);
    const rowB = makeCorridor("USDC->PHP", 97.5);

    const paginated: PaginatedResponse<CorridorMetrics> = {
      data: [rowA, rowB],
      pagination: {
        limit: 50,
        total: 2,
        has_next: false,
        has_prev: false,
        next_cursor: null,
        prev_cursor: null,
        offset: 0,
        next_offset: null,
        prev_offset: null,
      },
    };
    const listKey = queryKeys.corridorList({ sort_by: "health_score" });
    queryClient.setQueryData(listKey, paginated);

    const wrapper = createWrapper(queryClient);
    const { result } = renderHook(() => useRealtimeCorridors(), { wrapper });

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    act(() => {
      MockWebSocket.instances[0].emitServerMessage({
        type: "corridor_update",
        corridor_key: "USDC->NGN",
        success_rate: 99.2,
        health_score: 96.5,
      });
    });

    const updated =
      queryClient.getQueryData<PaginatedResponse<CorridorMetrics>>(listKey);
    expect(updated?.data[0].success_rate).toBe(99.2);
    expect(updated?.data[0].health_score).toBe(96.5);
    // Untouched sibling row retains EXACT object identity (zero unnecessary re-renders!).
    expect(updated?.data[1]).toBe(rowB);
  });

  it("patchCorridorInInfiniteData preserves untouched pages and rows", () => {
    const r1 = makeCorridor("c-1", 90);
    const r2 = makeCorridor("c-2", 91);
    const r3 = makeCorridor("c-3", 92);
    const page1: PaginatedResponse<CorridorMetrics> = {
      data: [r1, r2],
      pagination: {
        limit: 2,
        total: 3,
        has_next: true,
        has_prev: false,
        next_cursor: "cur-2",
        prev_cursor: null,
        offset: 0,
        next_offset: 2,
        prev_offset: null,
      },
    };
    const page2: PaginatedResponse<CorridorMetrics> = {
      data: [r3],
      pagination: {
        limit: 2,
        total: 3,
        has_next: false,
        has_prev: true,
        next_cursor: null,
        prev_cursor: "cur-0",
        offset: 2,
        next_offset: null,
        prev_offset: 0,
      },
    };

    const next = patchCorridorInInfiniteData(
      { pages: [page1, page2], pageParams: [undefined, "cur-2"] },
      { type: "corridor_update", corridor_key: "c-2", success_rate: 98.4 },
    );

    expect(next?.pages[0].data[1].success_rate).toBe(98.4);
    expect(next?.pages[0].data[0]).toBe(r1);
    expect(next?.pages[1]).toBe(page2);

    const noOp = patchCorridorInPaginatedData(page1, {
      type: "corridor_update",
      corridor_key: "does-not-exist",
      success_rate: 50,
    });
    expect(noOp).toBe(page1);
  });
});
