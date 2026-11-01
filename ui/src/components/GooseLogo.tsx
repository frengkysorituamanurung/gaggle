import React from "react";

export function GooseLogo({ className }: { className?: string }) {
  return (
    <span className={`font-bold text-text-primary tracking-tight ${className || ""}`}>
      Gaggle
    </span>
  );
}

export default GooseLogo;