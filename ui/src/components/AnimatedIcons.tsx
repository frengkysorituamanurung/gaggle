import React from "react";

export function AnimatedIcons({ children }: { children?: React.ReactNode }) {
  return <div className="inline-flex items-center gap-1 transition-transform">{children}</div>;
}

export default AnimatedIcons;