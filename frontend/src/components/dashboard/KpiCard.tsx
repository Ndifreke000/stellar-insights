import React from 'react';

interface KpiCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  className?: string;
}

export function KpiCard({ title, value, subtitle, className = '' }: KpiCardProps) {
  return (
    <div className={`col-span-1 bg-white rounded shadow p-4 ${className}`}>
      <h2 className="text-sm text-gray-500">{title}</h2>
      <div className="mt-3 flex items-end gap-4">
        <div className="text-4xl font-bold">{value}</div>
        {subtitle && <div className="text-sm text-gray-500">{subtitle}</div>}
      </div>
    </div>
  );
}
