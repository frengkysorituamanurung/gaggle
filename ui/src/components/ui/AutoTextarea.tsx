import React, { useEffect, useRef } from "react";
import { cn } from "../../lib/utils";

export interface AutoTextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  minRows?: number;
  maxRows?: number;
}

export const AutoTextarea = React.forwardRef<HTMLTextAreaElement, AutoTextareaProps>(
  ({ className, minRows = 1, maxRows = 10, value, ...props }, ref) => {
    const innerRef = useRef<HTMLTextAreaElement | null>(null);
    const setRef = (node: HTMLTextAreaElement | null) => {
      innerRef.current = node;
      if (typeof ref === "function") ref(node);
      else if (ref) (ref as React.MutableRefObject<HTMLTextAreaElement | null>).current = node;
    };

    useEffect(() => {
      const el = innerRef.current;
      if (!el) return;
      el.style.height = "auto";
      const lineHeight = parseInt(getComputedStyle(el).lineHeight) || 24;
      const minH = lineHeight * minRows;
      const maxH = lineHeight * maxRows;
      const next = Math.min(Math.max(el.scrollHeight, minH), maxH);
      el.style.height = `${next}px`;
      el.style.overflowY = el.scrollHeight > maxH ? "auto" : "hidden";
    }, [value, minRows, maxRows]);

    return (
      <textarea
        ref={setRef}
        rows={minRows}
        className={cn(
          "w-full px-3 py-2 rounded-xl border border-border-primary bg-background-primary text-text-primary text-sm resize-none focus:outline-none focus:ring-2 focus:ring-accent/40",
          className
        )}
        value={value}
        {...props}
      />
    );
  }
);
AutoTextarea.displayName = "AutoTextarea";