import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { TimeRangeSelector } from "../components/ui/TimeRangeSelector";

describe("TimeRangeSelector", () => {
  it("calls onChange when a preset is clicked", () => {
    const onChange = vi.fn();
    render(
      <TimeRangeSelector startDate={null} endDate={null} onChange={onChange} />,
    );

    const btn = screen.getByText("24h");
    fireEvent.click(btn);

    expect(onChange).toHaveBeenCalled();
  });

  it("shows date inputs when custom is selected and updates values", () => {
    const onChange = vi.fn();
    const { container } = render(
      <TimeRangeSelector startDate={null} endDate={null} onChange={onChange} />,
    );

    const custom = within(container).getByText("Custom");
    fireEvent.click(custom);

    const inputs = container.querySelectorAll(
      'input[type="date"]',
    ) as NodeListOf<HTMLInputElement>;

    expect(inputs.length).toBe(2);

    fireEvent.change(inputs[0], { target: { value: "2023-01-01" } });
    fireEvent.change(inputs[1], { target: { value: "2023-01-02" } });

    expect(onChange).toHaveBeenCalled();
  });
});
