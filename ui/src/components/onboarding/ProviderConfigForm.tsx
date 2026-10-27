import React from "react";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";
export default function ProviderConfigForm({ kind, onSubmit }: { kind: string; onSubmit: (cfg: any) => void }) {
  const [name, setName] = React.useState(kind);
  const [key, setKey] = React.useState("");
  const [url, setUrl] = React.useState("");
  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Configure {kind}</CardTitle></CardHeader>
      <CardContent>
        <div className="flex flex-col gap-2">
          <Input placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} />
          <Input placeholder="Base URL (optional)" value={url} onChange={(e) => setUrl(e.target.value)} />
          <Input placeholder="API key" value={key} onChange={(e) => setKey(e.target.value)} type="password" />
          <Button variant="default" size="sm" onClick={() => onSubmit({ kind, name, base_url: url || undefined, auth_kind: "api_key", api_key: key || undefined })}>Save</Button>
        </div>
      </CardContent>
    </Card>
  );
}