import * as React from "react";
import { Check } from "lucide-react";

export interface CheckboxProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange" | "type"> {
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
}

export const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className = "", checked, onCheckedChange, disabled, id, ...props }, ref) => {
    return (
      <label
        htmlFor={id}
        className={`inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-slate-300 shadow-sm cursor-pointer ${
          checked ? "bg-primary border-primary text-primary-foreground" : "bg-white dark:bg-slate-950"
        } ${disabled ? "cursor-not-allowed opacity-50" : ""} dark:border-slate-700 ${className}`}
      >
        <input
          ref={ref}
          id={id}
          type="checkbox"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onCheckedChange?.(e.target.checked)}
          className="peer sr-only"
          {...props}
        />
        {checked && <Check className="h-3 w-3" />}
      </label>
    );
  },
);
Checkbox.displayName = "Checkbox";
