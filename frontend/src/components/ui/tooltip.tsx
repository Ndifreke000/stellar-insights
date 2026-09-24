import * as React from "react";

export const TooltipProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return <>{children}</>;
};

interface TooltipContextProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const TooltipContext = React.createContext<TooltipContextProps | undefined>(undefined);

export const Tooltip: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [open, setOpen] = React.useState(false);
  return (
    <TooltipContext.Provider value={{ open, setOpen }}>
      <div className="relative inline-flex">{children}</div>
    </TooltipContext.Provider>
  );
};

export const TooltipTrigger: React.FC<{
  children: React.ReactElement;
  asChild?: boolean;
}> = ({ children }) => {
  const context = React.useContext(TooltipContext);
  return React.cloneElement(children, {
    onMouseEnter: (e: React.MouseEvent) => {
      context?.setOpen(true);
      (children.props as { onMouseEnter?: (e: React.MouseEvent) => void }).onMouseEnter?.(e);
    },
    onMouseLeave: (e: React.MouseEvent) => {
      context?.setOpen(false);
      (children.props as { onMouseLeave?: (e: React.MouseEvent) => void }).onMouseLeave?.(e);
    },
    onFocus: (e: React.FocusEvent) => {
      context?.setOpen(true);
      (children.props as { onFocus?: (e: React.FocusEvent) => void }).onFocus?.(e);
    },
    onBlur: (e: React.FocusEvent) => {
      context?.setOpen(false);
      (children.props as { onBlur?: (e: React.FocusEvent) => void }).onBlur?.(e);
    },
  } as React.HTMLAttributes<HTMLElement>);
};

export const TooltipContent: React.FC<{
  children: React.ReactNode;
  className?: string;
}> = ({ children, className = "" }) => {
  const context = React.useContext(TooltipContext);
  if (!context?.open) return null;

  return (
    <div
      className={`absolute bottom-full left-1/2 z-50 mb-2 -translate-x-1/2 whitespace-nowrap rounded-md border border-slate-200 bg-slate-900 px-3 py-1.5 text-xs text-white shadow-md animate-in fade-in-0 zoom-in-95 dark:border-slate-800 ${className}`}
    >
      {children}
    </div>
  );
};
