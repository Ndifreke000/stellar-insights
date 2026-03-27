"use client";

import { motion, useSpring, useTransform } from "framer-motion";
import { useEffect } from "react";
import { useLocale } from "next-intl";

const fontSize = 30;
const padding = 15;
const height = fontSize + padding;

interface AnimatedNumberProps {
  value: number;
  format?: "currency" | "percent" | "number" | "time";
}

export function AnimatedNumber({
  value,
  format = "number",
}: AnimatedNumberProps) {
  const locale = useLocale();
  const spring = useSpring(value, { mass: 0.8, stiffness: 75, damping: 15 });
  const display = useTransform(spring, (current) => {
    if (format === "currency") {
      return new Intl.NumberFormat(locale, {
        style: "currency",
        currency: "USD",
        maximumFractionDigits: 0,
      }).format(current);
    }
    if (format === "percent") {
      return `${current.toFixed(2)}%`;
    }
    if (format === "time") {
      return `${current.toFixed(0)} ms`;
    }
    return Math.round(current).toLocaleString(locale);
  });

  useEffect(() => {
    spring.set(value);
  }, [spring, value]);

  return <motion.span>{display}</motion.span>;
}
