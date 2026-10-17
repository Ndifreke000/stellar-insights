import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { axe, toHaveNoViolations } from "jest-axe";
import { CorridorComparisonTable } from "@/components/CorridorComparisonTable";
import {
  SuccessRateCompareChart,
  VolumeCompareChart,
  SlippageCompareChart,
} from "@/components/corridors/CorridorCompareCharts";
import { mockCorridors } from "@/components/lib/mockCorridorData";
import { generateMockCorridorData } from "@/lib/api/corridors";

expect.extend(toHaveNoViolations);

describe("Corridor Tables and Charts Accessibility", () => {
  it("CorridorComparisonTable has proper headers and no serious axe violations", async () => {
    const { container } = render(
      <CorridorComparisonTable corridors={mockCorridors.slice(0, 3)} />
    );

    // Verify proper th scopes
    const colHeaders = container.querySelectorAll("th[scope='col']");
    expect(colHeaders.length).toBeGreaterThan(0);

    const rowHeaders = container.querySelectorAll("th[scope='row']");
    expect(rowHeaders.length).toBeGreaterThan(0);

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it("CorridorCompareCharts provide accessible data tables with scope headers", () => {
    const detailData = [
      generateMockCorridorData("USDC:NGN"),
      generateMockCorridorData("EURC:BRL"),
    ];

    const { container: successContainer } = render(
      <SuccessRateCompareChart corridors={detailData} />
    );
    const table = successContainer.querySelector("table");
    expect(table).not.toBeNull();
    expect(table?.querySelector("caption")).not.toBeNull();
    expect(table?.querySelectorAll("th[scope='col']").length).toBeGreaterThan(0);
    expect(table?.querySelectorAll("th[scope='row']").length).toBeGreaterThan(0);

    const { container: volumeContainer } = render(
      <VolumeCompareChart corridors={detailData} />
    );
    expect(volumeContainer.querySelector("table")).not.toBeNull();

    const { container: slippageContainer } = render(
      <SlippageCompareChart corridors={detailData} />
    );
    expect(slippageContainer.querySelector("table")).not.toBeNull();
  });
});
