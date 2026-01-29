import React from "react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import { motion } from "framer-motion";

interface TrendIndicatorProps {
  trend: {
    value: number;
    direction: "up" | "down" | "neutral";
    isGood?: boolean;
  };
}

export function TrendIndicator({ trend }: TrendIndicatorProps) {
  const { value, direction, isGood = true } = trend;

  // improved color logic
  // if direction is neutral -> gray
  // if isGood is true -> up is green, down is red (unless up is bad?)
  // Actually, usually:
  // "Good" result = Green. "Bad" result = Red.
  // We pass `isGood` to explicitely say "this trend is positive/desirable".

  let colorClass = "text-gray-500";
  let Icon = Minus;

  if (direction === "up") {
    Icon = TrendingUp;
    colorClass = isGood ? "text-emerald-500" : "text-rose-500";
  } else if (direction === "down") {
    Icon = TrendingDown;
    colorClass = isGood ? "text-emerald-500" : "text-rose-500";
  }

  if (direction === "neutral") {
    return (
      <div className="flex items-center gap-1 text-xs font-medium text-gray-500 bg-gray-100 px-2 py-1 rounded-full">
        <Minus className="w-3 h-3" />
        <span>0%</span>
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      className={`flex items-center gap-1 text-xs font-medium ${colorClass} ${isGood ? "bg-emerald-50" : "bg-rose-50"} px-2 py-1 rounded-full`}
    >
      <Icon className="w-3 h-3" />
      <span>{Math.abs(value).toFixed(1)}%</span>
    </motion.div>
  );
}
