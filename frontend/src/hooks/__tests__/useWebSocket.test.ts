import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useWebSocket } from '@/hooks/useWebSocket';

// ── Minimal mock WebSocket ──────────────────────────────────────────────────
// Tracks every instance created so tests can grab the "current" socket and
// drive its lifecycle (onopen/onclose) like a real connection would.

class MockWebSocket {
  static OPEN = 1;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  readyState = MockWebSocket.OPEN;
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  sent: string[] = [];

  constructor(public url: string) {
    MockWebSocket.instances.push(this);
    queueMicrotask(() => this.onopen?.());
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }
}

beforeEach(() => {
  MockWebSocket.instances = [];
  vi.stubGlobal('WebSocket', MockWebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

function lastSocket(): MockWebSocket {
  return MockWebSocket.instances[MockWebSocket.instances.length - 1];
}

function subscribeMessages(ws: MockWebSocket): unknown[] {
  return ws.sent.map((s) => JSON.parse(s)).filter((m) => m.type === 'subscribe');
}

describe('useWebSocket – resubscribe after reconnect (#1782)', () => {
  it('resends subscribed channels when the connection reopens after a drop', async () => {
    const { result } = renderHook(() => useWebSocket('wss://example.test', { reconnectInterval: 5 }));

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    act(() => {
      result.current.subscribe(['corridor:US-NG', 'corridor:US-PH']);
    });

    const firstSocket = lastSocket();
    expect(subscribeMessages(firstSocket)).toEqual([
      { type: 'subscribe', channels: ['corridor:US-NG', 'corridor:US-PH'] },
    ]);

    // Simulate a dropped connection; the hook should open a new socket and
    // resubscribe to the channels that were active before the drop.
    act(() => {
      firstSocket.close();
    });

    await waitFor(() => expect(MockWebSocket.instances.length).toBe(2));
    const secondSocket = lastSocket();

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    expect(subscribeMessages(secondSocket)).toEqual([
      {
        type: 'subscribe',
        channels: expect.arrayContaining(['corridor:US-NG', 'corridor:US-PH']),
      },
    ]);
  });

  it('does not resubscribe to channels that were explicitly unsubscribed', async () => {
    const { result } = renderHook(() => useWebSocket('wss://example.test', { reconnectInterval: 5 }));

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    act(() => {
      result.current.subscribe(['corridor:US-NG', 'corridor:US-PH']);
    });
    act(() => {
      result.current.unsubscribe(['corridor:US-PH']);
    });

    const firstSocket = lastSocket();
    act(() => {
      firstSocket.close();
    });

    await waitFor(() => expect(MockWebSocket.instances.length).toBe(2));
    await waitFor(() => expect(result.current.isConnected).toBe(true));

    const resubscribe = subscribeMessages(lastSocket())[0] as { channels: string[] };
    expect(resubscribe.channels).toContain('corridor:US-NG');
    expect(resubscribe.channels).not.toContain('corridor:US-PH');
  });

  it('sends no subscribe message on (re)connect when nothing was ever subscribed', async () => {
    const { result } = renderHook(() => useWebSocket('wss://example.test', { reconnectInterval: 5 }));

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    expect(subscribeMessages(lastSocket())).toEqual([]);
  });
});
