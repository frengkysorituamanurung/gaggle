import React from "react";
import { GitBranch } from "lucide-react";

export function GitBranchIndicator({ branch }: { branch?: string | null }) {
  if (!branch) return null;
  return (
    <div className="flex items-center gap-1 text-xs text-text-secondary">
      <GitBranch className="w-3 h-3" />
      <span>{branch}</span>
    </div>
  );
}

export default GitBranchIndicator;<|end|><|start|>write to=assistant<|channel|>analysis<|message|>Created file successfully: ui/src/components/GitBranchIndicator.tsx