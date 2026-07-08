"use client";

import React from "react";
import { useFormContext } from "react-hook-form";
import { AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";

interface FormFieldProps {
  name: string;
  label: string;
  placeholder?: string;
  type?: "text" | "url" | "number" | "password" | "email";
  required?: boolean;
  description?: string;
  className?: string;
  children?: React.ReactNode;
}

export function FormField({
  name,
  label,
  placeholder,
  type = "text",
  required = false,
  description,
  className,
  children,
}: FormFieldProps) {
  const {
    register,
    formState: { errors },
  } = useFormContext();

  const error = errors[name];
  const errorId = error ? `error-${name}` : undefined;

  return (
    <div className={cn("space-y-2", className)}>
      <label
        htmlFor={name}
        className="block text-sm font-medium text-foreground"
      >
        {label}
        {required && <span className="text-red-500 ml-1">*</span>}
      </label>
      
      {children || (
        <input
          id={name}
          type={type}
          placeholder={placeholder}
          className={cn(
            "w-full rounded-xl bg-background/80 border px-4 py-2.5 text-foreground",
            "placeholder:text-muted-foreground focus:ring-2 focus:ring-accent/50",
            "transition-colors",
            error
              ? "border-red-500 focus:ring-red-500/50"
              : "border-border"
          )}
          aria-describedby={cn(
            errorId,
            description && `desc-${name}`
          )}
          aria-invalid={!!error}
          {...register(name)}
        />
      )}

      {description && (
        <p
          id={`desc-${name}`}
          className="text-xs text-muted-foreground"
        >
          {description}
        </p>
      )}

      {error && (
        <div
          id={errorId}
          className="flex items-center gap-1 text-xs text-red-400"
          role="alert"
          aria-live="polite"
        >
          <AlertCircle className="w-3 h-3" aria-hidden="true" />
          <span>{typeof error.message === 'string' ? error.message : 'Invalid input'}</span>
        </div>
      )}
    </div>
  );
}

interface FormSelectProps {
  name: string;
  label: string;
  options: Array<{ value: string; label: string }>;
  required?: boolean;
  description?: string;
  className?: string;
  placeholder?: string;
}

export function FormSelect({
  name,
  label,
  options,
  required = false,
  description,
  className,
  placeholder,
}: FormSelectProps) {
  const {
    register,
    formState: { errors },
  } = useFormContext();

  const error = errors[name];
  const errorId = error ? `error-${name}` : undefined;

  return (
    <div className={cn("space-y-2", className)}>
      <label
        htmlFor={name}
        className="block text-sm font-medium text-foreground"
      >
        {label}
        {required && <span className="text-red-500 ml-1">*</span>}
      </label>

      <select
        id={name}
        className={cn(
          "w-full rounded-xl bg-background/80 border px-4 py-2.5 text-foreground",
          "focus:ring-2 focus:ring-accent/50 transition-colors",
          error
            ? "border-red-500 focus:ring-red-500/50"
            : "border-border"
        )}
        aria-describedby={cn(
          errorId,
          description && `desc-${name}`
        )}
        aria-invalid={!!error}
        {...register(name)}
      >
        {placeholder && (
          <option value="" disabled>
            {placeholder}
          </option>
        )}
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>

      {description && (
        <p
          id={`desc-${name}`}
          className="text-xs text-muted-foreground"
        >
          {description}
        </p>
      )}

      {error && (
        <div
          id={errorId}
          className="flex items-center gap-1 text-xs text-red-400"
          role="alert"
          aria-live="polite"
        >
          <AlertCircle className="w-3 h-3" aria-hidden="true" />
          <span>{typeof error.message === 'string' ? error.message : 'Invalid selection'}</span>
        </div>
      )}
    </div>
  );
}

interface FormCheckboxProps {
  name: string;
  label: string;
  value: string;
  description?: string;
  className?: string;
}

export function FormCheckbox({
  name,
  label,
  value,
  description,
  className,
}: FormCheckboxProps) {
  const {
    watch,
    setValue,
    formState: { errors },
  } = useFormContext();

  const selectedValues = watch(name) || [];
  const isChecked = selectedValues.includes(value);
  const error = errors[name];
  const errorId = error ? `error-${name}` : undefined;

  const handleChange = () => {
    if (isChecked) {
      setValue(
        name,
        selectedValues.filter((v: string) => v !== value),
        { shouldValidate: true }
      );
    } else {
      setValue(name, [...selectedValues, value], { shouldValidate: true });
    }
  };

  return (
    <div className={cn("flex items-start gap-2", className)}>
      <input
        type="checkbox"
        id={`${name}-${value}`}
        checked={isChecked}
        onChange={handleChange}
        className="mt-0.5 rounded border-border text-accent focus:ring-accent/50"
        aria-describedby={cn(
          errorId,
          description && `desc-${name}-${value}`
        )}
        aria-invalid={!!error}
      />
      <div className="flex-1">
        <label
          htmlFor={`${name}-${value}`}
          className="text-sm font-medium text-foreground cursor-pointer"
        >
          {label}
        </label>
        {description && (
          <p
            id={`desc-${name}-${value}`}
            className="text-xs text-muted-foreground mt-1"
          >
            {description}
          </p>
        )}
      </div>
    </div>
  );
}

interface FormCheckboxGroupProps {
  name: string;
  label: string;
  options: Array<{ value: string; label: string; description?: string }>;
  required?: boolean;
  className?: string;
}

export function FormCheckboxGroup({
  name,
  label,
  options,
  required = false,
  className,
}: FormCheckboxGroupProps) {
  const {
    formState: { errors },
  } = useFormContext();

  const error = errors[name];
  const errorId = error ? `error-${name}` : undefined;

  return (
    <div className={cn("space-y-3", className)}>
      <div className="text-sm font-medium text-foreground">
        {label}
        {required && <span className="text-red-500 ml-1">*</span>}
      </div>
      
      <div
        role="group"
        aria-labelledby={label}
        aria-describedby={errorId}
        className="space-y-2"
      >
        {options.map((option) => (
          <FormCheckbox
            key={option.value}
            name={name}
            label={option.label}
            value={option.value}
            description={option.description}
          />
        ))}
      </div>

      {error && (
        <div
          id={errorId}
          className="flex items-center gap-1 text-xs text-red-400"
          role="alert"
          aria-live="polite"
        >
          <AlertCircle className="w-3 h-3" aria-hidden="true" />
          <span>{typeof error.message === 'string' ? error.message : 'Invalid selection'}</span>
        </div>
      )}
    </div>
  );
}
