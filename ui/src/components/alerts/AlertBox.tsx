import React from "react";

export function AlertBox({
  type = "info",
  message,
}: {
  type?: "info" | "warning" | "error" | "success";
  message: string;
}) {
  const color =
    type === "error" ? "text-text-danger" : type === "warning" ? "text-text-warning" : type === "success" ? "text-text-success" : "text-text-secondary";
  return (
    <div className={`rounded-md border border-border-primary bg-background-secondary px-3 py-2 text-sm ${color}`}>
      {message}
    </div>
  );
}

export default AlertBox;