import React from "react";
import { Button } from "../../ui/Button";

export function HuggingFaceSignInPrompt({ onSignIn }: { onSignIn?: () => void }) {
  return (
    <div className="rounded-md border border-border-primary bg-background-secondary p-3 text-sm">
      <div className="text-text-primary mb-2">Sign in to Hugging Face to access models.</div>
      <Button size="sm" variant="default" onClick={onSignIn}>Sign in</Button>
    </div>
  );
}

export default HuggingFaceSignInPrompt;