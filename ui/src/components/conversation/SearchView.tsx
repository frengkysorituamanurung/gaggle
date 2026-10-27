import React, { useState } from "react";
import { Input } from "../ui/Input";
export default function SearchView({ onSearch }: { onSearch?: (q: string) => void }) {
  const [q, setQ] = useState("");
  return <Input placeholder="Search messages…" value={q} onChange={(e) => { setQ(e.target.value); onSearch?.(e.target.value); }} />;
}