import React from "react";
import { Button } from "./ui/Button";
import { AudioLines } from "lucide-react";
export default function LiveVoiceButton() {
  return <Button size="sm" variant="secondary" title="Live voice (coming soon)"><AudioLines className="w-4 h-4" /></Button>;
}