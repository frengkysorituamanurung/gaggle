import React from "react";
import { cn } from "../../lib/utils";

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn("w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm focus:outline-none focus:ring-2 focus:ring-accent/40", className)}
      {...props}
    />
  )
);
Input.displayName = "Input";

export const Textarea = React.forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => (
    <textarea
      ref={ref}
      className={cn("w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm resize-y focus:outline-none focus:ring-2 focus:ring-accent/40", className)}
      {...props}
    />
  )
);
Textarea.displayName = "Textarea";