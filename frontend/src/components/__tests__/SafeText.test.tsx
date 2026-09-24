import React from "react";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SafeText } from "../SafeText";

describe("SafeText", () => {
  it("renders plain text as-is", () => {
    render(<SafeText value="Hello world" />);
    expect(screen.getByText("Hello world")).toBeInTheDocument();
  });

  it("strips HTML tags from the rendered output", () => {
    render(<SafeText value="<b>bold</b> and <script>alert(1)</script>text" />);
    expect(screen.getByText("bold and text")).toBeInTheDocument();
  });

  it("renders an empty string for null or undefined values", () => {
    const { container: nullContainer } = render(<SafeText value={null} />);
    expect(nullContainer.querySelector("span")?.textContent).toBe("");

    const { container: undefinedContainer } = render(<SafeText value={undefined} />);
    expect(undefinedContainer.querySelector("span")?.textContent).toBe("");
  });

  it("defaults to rendering a span", () => {
    const { container } = render(<SafeText value="text" />);
    expect(container.querySelector("span")).not.toBeNull();
  });

  it("renders the requested tag when `as` is provided", () => {
    const { container } = render(<SafeText value="text" as="p" />);
    expect(container.querySelector("p")).not.toBeNull();
    expect(container.querySelector("span")).toBeNull();
  });

  it("applies the provided className", () => {
    const { container } = render(<SafeText value="text" className="text-sm" />);
    expect(container.querySelector("span")).toHaveClass("text-sm");
  });
});
