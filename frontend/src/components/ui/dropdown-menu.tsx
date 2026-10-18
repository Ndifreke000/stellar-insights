import * as React from "react";

interface DropdownMenuContextProps {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const DropdownMenuContext = React.createContext<DropdownMenuContextProps | undefined>(undefined);

export const DropdownMenu: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [open, setOpen] = React.useState(false);
  return (
    <DropdownMenuContext.Provider value={{ open, setOpen }}>
      <div className="relative inline-block text-left">{children}</div>
    </DropdownMenuContext.Provider>
  );
};

export const DropdownMenuTrigger: React.FC<{
  children: React.ReactElement;
  asChild?: boolean;
}> = ({ children }) => {
  const context = React.useContext(DropdownMenuContext);
  return React.cloneElement(children, {
    onClick: (e: React.MouseEvent) => {
      context?.setOpen(!context.open);
      (children.props as { onClick?: (e: React.MouseEvent) => void }).onClick?.(e);
    },
  } as React.HTMLAttributes<HTMLElement>);
};

export const DropdownMenuContent: React.FC<{
  children: React.ReactNode;
  align?: "start" | "end" | "center";
  className?: string;
}> = ({ children, align = "start", className = "" }) => {
  const context = React.useContext(DropdownMenuContext);
  const contentRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (contentRef.current && !contentRef.current.contains(event.target as Node)) {
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

  const alignClass = align === "end" ? "right-0" : align === "center" ? "left-1/2 -translate-x-1/2" : "left-0";

  return (
    <div
      ref={contentRef}
      className={`absolute z-50 mt-2 min-w-[10rem] overflow-hidden rounded-md border border-slate-200 bg-white p-1 text-slate-950 shadow-md animate-in fade-in-0 zoom-in-95 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50 ${alignClass} ${className}`}
    >
      {children}
    </div>
  );
};

export const DropdownMenuItem = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & { className?: string }
>(({ className = "", onClick, children, ...props }, ref) => {
  const context = React.useContext(DropdownMenuContext);
  return (
    <div
      ref={ref}
      role="menuitem"
      onClick={(e) => {
        onClick?.(e);
        context?.setOpen(false);
      }}
      className={`relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-slate-100 dark:hover:bg-slate-800 ${className}`}
      {...props}
    >
      {children}
    </div>
  );
});
DropdownMenuItem.displayName = "DropdownMenuItem";

export const DropdownMenuSeparator: React.FC<{ className?: string }> = ({ className = "" }) => (
  <div className={`-mx-1 my-1 h-px bg-slate-200 dark:bg-slate-800 ${className}`} />
);
