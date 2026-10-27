import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import type * as T from "../types";
import { Button } from "../components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";

export default function WorkspaceScreen() {
  const { projectId } = useApp();
  const [files, setFiles] = useState<T.FsEntry[]>([]);
  const [gitStatus, setGitStatus] = useState<T.GitWorkspaceStatus | null>(null);
  const [diff, setDiff] = useState<T.GitDiffOutput | null>(null);
  const [worktrees, setWorktrees] = useState<T.Worktree[]>([]);
  const [path, setPath] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    if (!projectId) return;
    fetch(`/api/projects/${projectId}/files?path=${encodeURIComponent(path)}`).then((r) => r.json()).then(setFiles).catch((e) => setError(String(e)));
    fetch(`/api/projects/${projectId}/git-status`).then((r) => r.json()).then(setGitStatus).catch(() => {});
    fetch(`/api/projects/${projectId}/diff`).then((r) => r.json()).then(setDiff).catch(() => {});
    fetch(`/api/projects/${projectId}/worktrees`).then((r) => r.json()).then(setWorktrees).catch(() => {});
  };
  useEffect(refresh, [projectId, path]);

  const browse = (p?: string) => setPath(p || "");

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Workspace</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}
      {!projectId && <div className="text-text-secondary text-sm">Select a project first.</div>}
      {projectId && (
        <>
          <Card className="mb-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Files {path && `— ${path}`}</CardTitle></CardHeader>
            <CardContent>
              {path && <Button size="sm" variant="secondary" onClick={() => browse("")}>↑ Root</Button>}
              <ul className="mt-2 text-sm">
                {files.map((f) => (
                  <li key={f.relative_path} className="py-1">
                    {f.is_dir ? (
                      <button className="text-text-primary hover:underline" onClick={() => browse(f.relative_path)}>{f.relative_path.split("/").pop()}/</button>
                    ) : (
                      <span className="text-text-secondary">{f.relative_path.split("/").pop()}</span>
                    )}
                  </li>
                ))}
              </ul>
              {files.length === 0 && <div className="text-text-secondary text-sm">Empty.</div>}
            </CardContent>
          </Card>

          <Card className="mb-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Git Status</CardTitle></CardHeader>
            <CardContent>
              {gitStatus ? (
                <div className="text-sm text-text-secondary">
                  <div>branch: <span className="text-text-primary">{gitStatus.branch}</span></div>
                  <div>clean: {gitStatus.is_clean ? "yes" : "no"}</div>
                  {gitStatus.changed_files && gitStatus.changed_files.length > 0 && (
                    <ul className="mt-2">
                      {gitStatus.changed_files.map((e, i) => (
                        <li key={i} className="text-xs"><code>{e.index_status}{e.worktree_status}</code> {e.path}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ) : <div className="text-text-secondary text-sm">No git info.</div>}
            </CardContent>
          </Card>

          <Card className="mb-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Diff</CardTitle></CardHeader>
            <CardContent>
              {diff?.patch ? (
                <pre className="text-xs bg-code-bg p-3 rounded-md overflow-x-auto max-h-96">{diff.patch}</pre>
              ) : <div className="text-text-secondary text-sm">No changes.</div>}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-text-primary text-base">Worktrees</CardTitle></CardHeader>
            <CardContent>
              {worktrees.length === 0 && <div className="text-text-secondary text-sm">No worktrees.</div>}
              <ul className="text-sm">
                {worktrees.map((w) => (
                  <li key={w.id} className="py-1 border-b border-border-primary last:border-0">
                    <div className="text-text-primary">{w.branch}</div>
                    <div className="text-xs text-text-secondary">{w.path}</div>
                    <div className="text-xs text-text-tertiary">base: {w.base_ref} @ {w.base_commit?.slice(0, 8)}</div>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}