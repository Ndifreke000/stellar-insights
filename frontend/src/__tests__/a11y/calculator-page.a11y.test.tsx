/**
 * Page-level accessibility test.
 *
 * The CI accessibility workflow (.github/workflows/accessibility.yml) has run
 * `src/__tests__/a11y` alongside the component-level suite for a while, but
 * no test file lived under that path, so it silently matched nothing. This
 * is the first real page-level WCAG AA check; add more page tests here.
 */
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { axe, toHaveNoViolations } from "jest-axe";
import CalculatorPage from "@/app/[locale]/calculator/page";

expect.extend(toHaveNoViolations);

describe("Calculator page accessibility", () => {
  it("should not have accessibility violations", async () => {
    const { container } = render(<CalculatorPage />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it("should have a single top-level heading", () => {
    const { getAllByRole } = render(<CalculatorPage />);
    const headings = getAllByRole("heading", { level: 2 });
    expect(headings).toHaveLength(1);
    expect(headings[0]).toHaveTextContent(/cost calculator/i);
  });
});
