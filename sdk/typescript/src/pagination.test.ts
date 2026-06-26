import { describe, it, expect, vi, beforeEach } from "vitest";
import { paginate, paginateAll } from "./pagination.js";
import type { HttpClient } from "./http.js";

describe("pagination", () => {
  let mockHttpClient: HttpClient;

  beforeEach(() => {
    mockHttpClient = {
      request: vi.fn(),
    } as unknown as HttpClient;
  });

  it("normal pagination terminates correctly", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest
      .mockResolvedValueOnce({
        data: [{ id: 1 }, { id: 2 }],
        cursor: "cursor2",
      })
      .mockResolvedValueOnce({
        data: [{ id: 3 }, { id: 4 }],
        cursor: "cursor3",
      })
      .mockResolvedValueOnce({
        data: [{ id: 5 }], // Less than page size, terminates
        cursor: "cursor4",
      });

    const results: Array<{ id: number }> = [];
    for await (const page of paginate(mockHttpClient, "/api/test", 2)) {
      results.push(...page.data);
    }

    expect(results).toHaveLength(5);
    expect(mockRequest).toHaveBeenCalledTimes(3);
  });

  it("detects cursor wrap-around and breaks loop", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest
      .mockResolvedValueOnce({
        data: [{ id: 1 }, { id: 2 }],
        cursor: "cursor_a",
      })
      .mockResolvedValueOnce({
        data: [{ id: 3 }, { id: 4 }],
        cursor: "cursor_b",
      })
      .mockResolvedValueOnce({
        data: [{ id: 5 }, { id: 6 }],
        cursor: "cursor_a", // Wrap-around: cursor_a seen again
      });

    const results: Array<{ id: number }> = [];
    for await (const page of paginate(mockHttpClient, "/api/test", 2)) {
      results.push(...page.data);
    }

    // Should have data from first 3 requests (6 items total)
    expect(results).toHaveLength(6);
    // Should not continue after wrap-around is detected
    expect(mockRequest).toHaveBeenCalledTimes(3);
  });

  it("handles empty result set gracefully", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest.mockResolvedValueOnce({
      data: [],
      cursor: undefined,
    });

    const results: Array<{ id: number }> = [];
    for await (const page of paginate(mockHttpClient, "/api/test", 10)) {
      results.push(...page.data);
    }

    expect(results).toHaveLength(0);
    expect(mockRequest).toHaveBeenCalledTimes(1);
  });

  it("terminates when no cursor in response", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest
      .mockResolvedValueOnce({
        data: [{ id: 1 }, { id: 2 }],
        cursor: "cursor1",
      })
      .mockResolvedValueOnce({
        data: [{ id: 3 }],
        // No cursor in response
      });

    const results: Array<{ id: number }> = [];
    for await (const page of paginate(mockHttpClient, "/api/test", 2)) {
      results.push(...page.data);
    }

    expect(results).toHaveLength(3);
    expect(mockRequest).toHaveBeenCalledTimes(2);
  });

  it("paginateAll collects all results", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest
      .mockResolvedValueOnce({
        data: [{ id: 1 }, { id: 2 }],
        cursor: "cursor1",
      })
      .mockResolvedValueOnce({
        data: [{ id: 3 }, { id: 4 }],
        // No cursor, terminates
      });

    const results = await paginateAll(mockHttpClient, "/api/test", 2);

    expect(results).toHaveLength(4);
    expect(results.map((r) => r.id)).toEqual([1, 2, 3, 4]);
  });

  it("passes pagination params to request", async () => {
    const mockRequest = mockHttpClient.request as ReturnType<typeof vi.fn>;
    mockRequest.mockResolvedValueOnce({
      data: [{ id: 1 }],
      // No cursor, terminates after first page
    });

    for await (const _page of paginate(mockHttpClient, "/api/test", 50, {
      params: { sort: "id", order: "asc" },
    })) {
      // consume
    }

    const [, , options] = mockRequest.mock.calls[0];
    expect(options.params).toMatchObject({
      limit: 50,
      sort: "id",
      order: "asc",
    });
  });
});
