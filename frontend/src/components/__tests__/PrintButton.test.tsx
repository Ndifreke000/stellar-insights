import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PrintButton } from "../PrintButton";

describe("PrintButton", () => {
  let printSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    printSpy = vi.spyOn(window, "print").mockImplementation(() => {});
  });

  afterEach(() => {
    printSpy.mockRestore();
    document.documentElement.classList.remove("theme-transition");
  });

  it("renders the default label", () => {
    render(<PrintButton />);
    expect(screen.getByRole("button", { name: "Print this page" })).toHaveTextContent("Print");
  });

  it("renders a custom label when provided", () => {
    render(<PrintButton label="Export as PDF" />);
    expect(screen.getByText("Export as PDF")).toBeInTheDocument();
  });

  it("calls window.print when clicked", () => {
    render(<PrintButton />);
    fireEvent.click(screen.getByRole("button", { name: "Print this page" }));
    expect(printSpy).toHaveBeenCalledTimes(1);
  });

  it("calls onBeforePrint before printing", () => {
    const onBeforePrint = vi.fn();
    render(<PrintButton onBeforePrint={onBeforePrint} />);
    fireEvent.click(screen.getByRole("button", { name: "Print this page" }));
    expect(onBeforePrint).toHaveBeenCalledTimes(1);
  });

  it("calls onAfterPrint when the afterprint event fires", () => {
    const onAfterPrint = vi.fn();
    render(<PrintButton onAfterPrint={onAfterPrint} />);
    fireEvent.click(screen.getByRole("button", { name: "Print this page" }));

    window.dispatchEvent(new Event("afterprint"));

    expect(onAfterPrint).toHaveBeenCalledTimes(1);
  });

  it("removes the theme-transition class before printing", () => {
    document.documentElement.classList.add("theme-transition");
    render(<PrintButton />);
    fireEvent.click(screen.getByRole("button", { name: "Print this page" }));
    expect(document.documentElement.classList.contains("theme-transition")).toBe(false);
  });

  it("applies extra className to the button", () => {
    render(<PrintButton className="extra-class" />);
    expect(screen.getByRole("button", { name: "Print this page" })).toHaveClass("extra-class");
  });
});
