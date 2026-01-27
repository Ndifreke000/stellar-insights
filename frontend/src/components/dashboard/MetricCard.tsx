import React from "react";
import { motion } from "framer-motion";
import { AnimatedNumber } from "./AnimatedNumber";
import { TrendIndicator } from "./TrendIndicator";
import { LucideIcon } from "lucide-react";

interface MetricCardProps {
  title: string;
  value: number;
  format?: "currency" | "percent" | "number" | "time";
  trend?: {
    value: number;
    direction: "up" | "down" | "neutral";
    isGood: boolean;
  };
  icon: LucideIcon;
  subtitle?: string;
  className?: string;
  delay?: number;
}

export function MetricCard({
  title,
  value,
  format = "number",
  trend,
  icon: Icon,
  subtitle,
  className = "",
  delay = 0,
}: MetricCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay }}
      className={`bg-white rounded-xl shadow-sm border border-gray-100 p-6 hover:shadow-md transition-shadow relative overflow-hidden ${className}`}
    >
      {/* Background Decoration */}
      <div className="absolute top-0 right-0 p-4 opacity-5">
        <Icon className="w-24 h-24 text-gray-900 rotate-12" />
      </div>

      <div className="relative z-10 flex flex-col h-full justify-between">
        <div className="flex items-start justify-between mb-4">
          <div className="p-2 bg-blue-50 rounded-lg text-blue-600">
            <Icon className="w-5 h-5" />
          </div>
          {trend && <TrendIndicator trend={trend} />}
        </div>

        <div>
          <h3 className="text-sm font-medium text-gray-500 mb-1">{title}</h3>
          <div className="text-2xl font-bold text-gray-900 tracking-tight">
            <AnimatedNumber value={value} format={format} />
          </div>
          {subtitle && <p className="text-xs text-gray-400 mt-1">{subtitle}</p>}
        </div>
      </div>
    </motion.div>
  );
}
