import React from "react";
import {
  Terminal,
  Search,
  FileText,
  FileEdit,
  Globe,
  Brain,
  Eye,
  Monitor,
  FilePlus,
  Save,
  Camera,
  Users,
  Hash,
  Archive,
  Settings,
  Code2,
  Paperclip,
  Wrench,
} from "lucide-react";

const ICON_MAP: Record<string, React.ComponentType<{ className?: string }>> = {
  terminal: Terminal,
  search: Search,
  file_text: FileText,
  file_edit: FileEdit,
  globe: Globe,
  brain: Brain,
  eye: Eye,
  monitor: Monitor,
  file_plus: FilePlus,
  save: Save,
  camera: Camera,
  delegate: Users,
  numbers: Hash,
  archive: Archive,
  settings: Settings,
  code: Code2,
  attach: Paperclip,
};

export function toolIconFor(name?: string): React.ReactNode {
  if (!name) return <Wrench className="w-4 h-4" />;
  const key = name.toLowerCase().replace(/[^a-z0-9]+/g, "_");
  const Icon = ICON_MAP[key] || ICON_MAP[name.toLowerCase()] || Wrench;
  return <Icon className="w-4 h-4" />;
}

export default toolIconFor;