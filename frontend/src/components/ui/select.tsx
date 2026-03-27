import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, Check } from "lucide-react";

interface SelectContextProps {
  value?: string;
  onValueChange?: (value: any) => void;
  open: boolean;
  setOpen: (open: boolean) => void;
}

const SelectContext = React.createContext<SelectContextProps | undefined>(
  undefined,
);

export const Select: React.FC<{
  children: React.ReactNode;
  value?: string;
  onValueChange?: (value: any) => void;
}> = ({ children, value, onValueChange }) => {
  const [open, setOpen] = useState(false);
  return (
    <SelectContext.Provider value={{ value, onValueChange, open, setOpen }}>
      <div className="relative w-full">{children}</div>
    </SelectContext.Provider>
  );
};

export const SelectTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ButtonHTMLAttributes<HTMLButtonElement>
>(({ className = "", children, ...props }, ref) => {
  const context = React.useContext(SelectContext);
  return (
    <button
      ref={ref}
      type="button"
      onClick={() => context?.setOpen(!context.open)}
      className={`flex h-9 w-full items-center justify-between rounded-md border border-slate-200 bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-white placeholder:text-slate-500 focus:outline-none focus:ring-1 focus:ring-slate-400 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-800 dark:ring-offset-slate-950 dark:placeholder:text-slate-400 dark:focus:ring-slate-300 ${className}`}
      {...props}
    >
      {children}
      <ChevronDown className="h-4 w-4 opacity-50" />
    </button>
  );
});
SelectTrigger.displayName = "SelectTrigger";

export const SelectValue: React.FC<{
  placeholder?: string;
  className?: string;
}> = ({ placeholder, className = "" }) => {
  const context = React.useContext(SelectContext);
  return (
    <span className={`block truncate ${className}`}>
      {context?.value || placeholder}
    </span>
  );
};

export const SelectContent: React.FC<{
  children: React.ReactNode;
  className?: string;
}> = ({ children, className = "" }) => {
  const context = React.useContext(SelectContext);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        contentRef.current &&
        !contentRef.current.contains(event.target as Node)
      ) {
        context?.setOpen(false);
      }
    };
    if (context?.open) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [context]);

  if (!context?.open) return null;

  return (
    <div
      ref={contentRef}
      className={`absolute z-50 mt-1 max-h-60 w-full overflow-auto rounded-md border border-slate-200 bg-white p-1 text-slate-950 shadow-md animate-in fade-in-0 zoom-in-95 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50 ${className}`}
    >
      {children}
    </div>
  );
};

export const SelectItem = React.forwardRef<
  HTMLDivElement,
  { value: string; children: React.ReactNode; className?: string }
>(({ value, children, className = "", ...props }, ref) => {
  const context = React.useContext(SelectContext);
  const isSelected = context?.value === value;

  return (
    <div
      ref={ref}
      className={`relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none hover:bg-slate-100 focus:bg-slate-100 dark:hover:bg-slate-800 dark:focus:bg-slate-800 ${className}`}
      onClick={() => {
        context?.onValueChange?.(value);
        context?.setOpen(false);
      }}
      {...props}
    >
      <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        {isSelected && <Check className="h-4 w-4" />}
      </span>
      <span className="truncate">{children}</span>
    </div>
  );
});
SelectItem.displayName = "SelectItem";
