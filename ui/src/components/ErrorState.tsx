import React from "react";

export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center py-8 text-text-danger text-sm gap-2">
      <div>{message}</div>
      {onRetry && (
        <button className="text-text-primary underline" onClick={onRetry}>
          Retry
        </button>
      )}
    </div>
  );
}
