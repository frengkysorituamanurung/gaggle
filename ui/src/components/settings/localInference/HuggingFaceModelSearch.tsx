import React, { useState } from "react";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export function HuggingFaceModelSearch() {
  const [query, setQuery] = useState("");
  return (
    <div className="flex gap-2">
      <Input placeholder="Search Hugging Face models" value={query} onChange={(e) => setQuery(e.target.value)} />
      <Button size="sm" variant="secondary">Search</Button>
    </div>
  );
}

export default HuggingFaceModelSearch;