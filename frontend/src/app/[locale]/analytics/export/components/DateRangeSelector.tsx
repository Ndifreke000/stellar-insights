import React from "react";
import { TimeRangeSelector } from "@/components/ui/TimeRangeSelector";

interface DateRangeSelectorProps {
  startDate: Date | null;
  endDate: Date | null;
  onChange: (start: Date | null, end: Date | null) => void;
}

export function DateRangeSelector({
  startDate,
  endDate,
  onChange,
}: DateRangeSelectorProps) {
  return (
    <TimeRangeSelector
      startDate={startDate}
      endDate={endDate}
      onChange={onChange}
    />
  );
}
