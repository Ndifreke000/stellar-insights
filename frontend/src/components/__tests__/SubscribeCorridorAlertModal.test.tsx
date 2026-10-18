import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SubscribeCorridorAlertModal } from "../corridors/SubscribeCorridorAlertModal";
import { corridorAlertsApi } from "@/lib/alerts-api";

vi.mock("@/lib/alerts-api", () => ({
  corridorAlertsApi: {
    createConfig: vi.fn(),
  },
}));

describe("SubscribeCorridorAlertModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not render when isOpen is false", () => {
    const { container } = render(
      <SubscribeCorridorAlertModal
        corridorKey="USDC:TEST->XLM"
        isOpen={false}
        onClose={vi.fn()}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders modal form when isOpen is true", () => {
    render(
      <SubscribeCorridorAlertModal
        corridorKey="USDC:TEST->XLM"
        isOpen={true}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByText(/Subscribe to Corridor Alerts/i)).toBeInTheDocument();
    expect(screen.getByText(/USDC:TEST->XLM/i)).toBeInTheDocument();
    expect(screen.getByText(/Corridor Status Changes/i)).toBeInTheDocument();
    expect(screen.getByText(/Email Notifications/i)).toBeInTheDocument();
  });

  it("validates that email is provided when email channel is enabled", async () => {
    render(
      <SubscribeCorridorAlertModal
        corridorKey="USDC:TEST->XLM"
        isOpen={true}
        onClose={vi.fn()}
      />
    );

    const submitBtn = screen.getByRole("button", { name: /^Subscribe$/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(
        screen.getByText(/Please provide an email address for email notifications/i)
      ).toBeInTheDocument();
    });
  });

  it("submits subscription configuration successfully", async () => {
    vi.mocked(corridorAlertsApi.createConfig).mockResolvedValueOnce({
      id: "cfg-123",
      user_id: "user-1",
      corridor_key: "USDC:TEST->XLM",
      name: "Status Alerts for USDC:TEST->XLM",
      success_rate_drop_pct: 10,
      latency_increase_pct: 50,
      liquidity_drop_pct: 30,
      cooldown_seconds: 300,
      notify_email: true,
      notify_webhook: false,
      notify_in_app: true,
      notify_slack: false,
      notify_telegram: false,
      is_active: true,
      created_at: "2026-10-17T12:00:00Z",
      updated_at: "2026-10-17T12:00:00Z",
    });

    render(
      <SubscribeCorridorAlertModal
        corridorKey="USDC:TEST->XLM"
        isOpen={true}
        onClose={vi.fn()}
      />
    );

    const emailInput = screen.getByPlaceholderText("you@example.com");
    fireEvent.change(emailInput, { target: { value: "operator@stellar.org" } });

    const submitBtn = screen.getByRole("button", { name: /^Subscribe$/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(corridorAlertsApi.createConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          corridor_key: "USDC:TEST->XLM",
          email: "operator@stellar.org",
          notify_email: true,
          alert_on_status_change: true,
        })
      );
      expect(screen.getByText(/Subscribed Successfully!/i)).toBeInTheDocument();
    });
  });
});
