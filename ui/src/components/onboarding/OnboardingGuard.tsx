import React, { useEffect, useState } from "react";
import { useApp } from "../../lib/store";
import { api } from "../../lib/api";
import type * as T from "../../types";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/Card";

export default function OnboardingGuard({ children }: { children: React.ReactNode }) {
  const { projectId, setProjectId } = useApp();
  const [projects, setProjects] = useState<T.ProjectStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.projects.list().then((ps) => {
      setProjects(ps);
      if (!projectId && ps.length) setProjectId(ps[0].id);
    }).finally(() => setLoading(false));
  }, []);

  const create = async () => {
    if (!name.trim() || !path.trim()) return;
    try {
      const p = await api.projects.create(name.trim(), path.trim());
      setProjects((prev) => [...prev, p]);
      setProjectId(p.id);
    } catch (e) { setError(String(e)); }
  };

  if (loading) return <div className="flex items-center justify-center h-full text-text-secondary">Loading…</div>;
  if (projects.length === 0) {
    return (
      <div className="flex items-center justify-center h-full p-6">
        <Card className="w-full max-w-md">
          <CardHeader><CardTitle className="text-text-primary">Welcome to Gaggle</CardTitle></CardHeader>
          <CardContent>
            <p className="text-text-secondary text-sm mb-3">Register a project to get started.</p>
            {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
            <div className="flex flex-col gap-2">
              <Input placeholder="Project name" value={name} onChange={(e) => setName(e.target.value)} />
              <Input placeholder="Root path (absolute)" value={path} onChange={(e) => setPath(e.target.value)} />
              <Button variant="default" onClick={create}>Create Project</Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }
  return <>{children}</>;
}