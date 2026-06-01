import React from 'react';

interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'secondary' | 'destructive' | 'outline' | 'success' | 'warning';
}

export const Badge: React.FC<BadgeProps> = ({ children, variant = 'default', className = '', ...props }) => {
  const variants = {
    default: 'bg-slate-100 text-slate-900 dark:bg-slate-800 dark:text-slate-100',
    secondary: 'bg-slate-100 text-slate-900 dark:bg-slate-800 dark:text-slate-100',
    destructive: 'bg-red-100 text-red-900 dark:bg-red-900/20 dark:text-red-400',
    outline: 'border border-slate-200 text-slate-900 dark:border-slate-800 dark:text-slate-100',
    success: 'bg-green-100 text-green-900 dark:bg-green-900/20 dark:text-green-400',
    warning: 'bg-yellow-100 text-yellow-900 dark:bg-yellow-900/20 dark:text-yellow-400',
  };

  return (
    <div
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-slate-400 ${variants[variant]} ${className}`}
      {...props}
    >
      {children}
    </div>
  );
};
