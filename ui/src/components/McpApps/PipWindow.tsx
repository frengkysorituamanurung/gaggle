import React from "react";
export default function PipWindow({ children }: { children: React.ReactNode }) {
  return <div className="fixed bottom-4 right-4 w-80 border border-border-primary rounded-xl shadow-lg bg-background-primary p-3 z-50">{children}</div>;
}