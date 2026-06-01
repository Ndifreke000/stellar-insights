"use client";

import React from "react";

interface DiceAnimationProps {
  value?: number;
  isRolling?: boolean;
}

const diceFaces = [
  [[4]], // 1
  [[0, 8]], // 2
  [[0, 4, 8]], // 3
  [[0, 2, 6, 8]], // 4
  [[0, 2, 4, 6, 8]], // 5
  [[0, 2, 3, 5, 6, 8]], // 6
];

export function DiceAnimation({ value = 1, isRolling = false }: DiceAnimationProps) {
  const dots = diceFaces[value - 1] || diceFaces[0];

  return (
    <div
      className="relative flex items-center justify-center"
      role="img"
      aria-label={isRolling ? "Rolling dice" : `Dice showing ${value}`}
    >
      <div
        className={`relative h-20 w-20 rounded-xl border-2 border-accent/40 bg-slate-900/80 shadow-lg ${
          isRolling ? "animate-spin" : ""
        }`}
        style={{
          animationDuration: isRolling ? "0.5s" : undefined,
        }}
      >
        <div className="grid grid-cols-3 grid-rows-3 gap-1 p-3 h-full">
          {[0, 1, 2, 3, 4, 5, 6, 7, 8].map((i) => (
            <div
              key={i}
              className={`rounded-full transition-all ${
                dots.flat().includes(i)
                  ? "bg-accent scale-100"
                  : "bg-transparent scale-0"
              }`}
            />
          ))}
        </div>
      </div>
      <div
        className="sr-only"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {isRolling ? "Rolling..." : `Result: ${value}`}
      </div>
    </div>
  );
}
