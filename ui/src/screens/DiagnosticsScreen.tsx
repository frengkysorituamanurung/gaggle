import React, { useEffect, useState } from "react";
import { api } from "../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";

interface DiagnosticsScreenProps {
  projectId?: string;
}

export default function DiagnosticsScreen({ projectId }: DiagnosticsScreenProps) {
  const [diagnostics, setDiagnostics] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDiagnostics = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.diagnostics();
      setDiagnostics(result as Record<string, unknown>);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadDiagnostics();
  }, []);

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Diagnostics</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      <Button size="sm" onClick={loadDiagnostics} disabled={loading} className="mb-4">
        {loading ? "Loading..." : "Refresh"}
      </Button>
      {diagnostics && (
        <Card>
          <CardHeader>
            <CardTitle className="text-text-primary text-base">System Health</CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-xs text-text-secondary whitespace-pre-wrap overflow-auto max-h-[60vh]">
              {JSON.stringify(diagnostics, null, 2)}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  );
}