"use client";

import React, { useState } from "react";
import { MainLayout } from "@/components/layout";
import TimeRangeSelector from "@/components/ui/TimeRangeSelector";
import { startOfDay } from "date-fns";

export default function TimeRangeDemoPage() {
  const [start, setStart] = useState<Date | null>(startOfDay(new Date()));
  const [end, setEnd] = useState<Date | null>(new Date());

  return (
    <MainLayout>
      <div className="p-6 max-w-4xl mx-auto">
        <h1 className="text-2xl font-bold mb-4">Time Range Selector Demo</h1>

        <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
          <TimeRangeSelector
            startDate={start}
            endDate={end}
            onChange={(s, e) => {
              setStart(s);
              setEnd(e);
            }}
          />

          <div className="mt-6 text-sm text-gray-700 dark:text-gray-300">
            <div>
              <strong>Selected Start:</strong>{" "}
              {start ? start.toISOString() : "-"}
            </div>
            <div>
              <strong>Selected End:</strong> {end ? end.toISOString() : "-"}
            </div>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}
