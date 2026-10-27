import React, { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { api } from "../lib/api";
import type * as T from "../types";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../components/ui/Tabs";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Switch } from "../components/ui/Switch";
import ThemeSelector from "../components/GooseSidebar/ThemeSelector";
import ModelsSection from "../components/settings/models/ModelsSection";
import ModeSection from "../components/settings/chat/ModeSection";
import GoosehintsSection from "../components/settings/chat/GoosehintsSection";
import LiveVoiceSettings from "../components/settings/chat/LiveVoiceSettings";
import DictationSettings from "../components/settings/chat/DictationSettings";
import ResponseStylesSection from "../components/settings/chat/ResponseStylesSection";
import SecurityToggle from "../components/settings/chat/SecurityToggle";
import SpellcheckToggle from "../components/settings/chat/SpellcheckToggle";
import AgentLoopSettings from "../components/settings/agent/AgentLoopSettings";
import ExternalBackendSection from "../components/settings/agent/ExternalBackendSection";
import PromptsSettingsSection from "../components/settings/prompts/PromptsSettingsSection";
import AuthSettingsSection from "../components/settings/auth/AuthSettingsSection";
import AppSettingsSection from "../components/settings/app/AppSettingsSection";
import LocalInferenceSection from "../components/settings/localInference/LocalInferenceSection";

export default function SettingsScreen() {
  const { theme, setTheme } = useApp();
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [settings, setSettings] = useState<T.AppSetting[]>([]);
  const [approvals, setApprovals] = useState<T.Approval[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [pName, setPName] = useState("");
  const [pKind, setPKind] = useState("openai_compat");
  const [pAuth, setPAuth] = useState("api_key");
  const [pKey, setPKey] = useState("");
  const [pUrl, setPUrl] = useState("");

  const refresh = () => {
    api.providers.list().then(setProviders).catch(() => {});
    api.settings.list().then(setSettings).catch(() => {});
    api.approvals.pending().then(setApprovals).catch(() => {});
  };
  useEffect(refresh, []);

  const createProvider = async () => {
    try {
      await api.providers.create({ kind: pKind, name: pName || pKind, base_url: pUrl || undefined, auth_kind: pAuth, api_key: pKey || undefined });
      setPName(""); setPKey(""); setPUrl("");
      refresh();
    } catch (e) { setError(String(e)); }
  };

  const decide = async (id: string, approved: boolean) => {
    await api.approvals.decide(id, approved);
    refresh();
  };

  const inputCls = "w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm";

  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Settings</h2>
      {error && <div className="text-text-danger text-sm mb-2">{error}</div>}

      <Tabs defaultValue="models">
        <TabsList>
          <TabsTrigger value="models">Models</TabsTrigger>
          <TabsTrigger value="local-inference">Local Inference</TabsTrigger>
          <TabsTrigger value="chat">Chat</TabsTrigger>
          <TabsTrigger value="agent">Agent</TabsTrigger>
          <TabsTrigger value="prompts">Prompts</TabsTrigger>
          <TabsTrigger value="keyboard">Keyboard</TabsTrigger>
          <TabsTrigger value="auth">Auth</TabsTrigger>
          <TabsTrigger value="app">App</TabsTrigger>
        </TabsList>

        <TabsContent value="models">
          <ModelsSection />
          <Card className="mt-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Add Provider</CardTitle></CardHeader>
            <CardContent>
              <div className="flex flex-col gap-2">
                <Input placeholder="Name" value={pName} onChange={(e) => setPName(e.target.value)} />
                <select className={inputCls} value={pKind} onChange={(e) => setPKind(e.target.value)}>
                  {["openai","anthropic","openai_compat","ollama","chatgpt","openrouter","deepseek","xai","google","azure","databricks","snowflake"].map((k) => (<option key={k} value={k}>{k}</option>))}
                </select>
                <select className={inputCls} value={pAuth} onChange={(e) => setPAuth(e.target.value)}>
                  <option value="api_key">api_key</option>
                  <option value="oauth">oauth</option>
                  <option value="none">none</option>
                </select>
                <Input placeholder="Base URL (optional)" value={pUrl} onChange={(e) => setPUrl(e.target.value)} />
                <Input placeholder="API key (stored securely)" value={pKey} onChange={(e) => setPKey(e.target.value)} type="password" />
                <Button variant="default" size="sm" onClick={createProvider}>Add Provider</Button>
              </div>
              <div className="mt-3">
                {providers.map((p) => (
                  <div key={p.provider.id} className="py-2 border-b border-border-primary last:border-0">
                    <span className="font-medium text-text-primary">{p.provider.name}</span>{" "}
                    <span className="text-xs px-2 py-0.5 rounded-full bg-surface-2 text-text-secondary">{p.provider.kind}</span>
                    <div className="text-xs text-text-secondary">secret configured: {p.has_configured_secret ? "yes" : "no"}</div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
          <Card className="mt-4">
            <CardHeader><CardTitle className="text-text-primary text-base">Appearance</CardTitle></CardHeader>
            <CardContent>
              <div className="flex gap-2"><ThemeSelector /></div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="local-inference"><LocalInferenceSection /></TabsContent>

        <TabsContent value="chat">
          <div className="flex flex-col gap-4">
            <ModeSection />
            <GoosehintsSection />
            <ResponseStylesSection />
            <SecurityToggle />
            <SpellcheckToggle />
            <LiveVoiceSettings />
            <DictationSettings />
          </div>
        </TabsContent>

        <TabsContent value="agent">
          <div className="flex flex-col gap-4">
            <AgentLoopSettings />
            <ExternalBackendSection />
          </div>
        </TabsContent>

        <TabsContent value="prompts"><PromptsSettingsSection /></TabsContent>

        <TabsContent value="keyboard">
          <Card>
            <CardHeader><CardTitle className="text-text-primary text-base">Keyboard Shortcuts</CardTitle></CardHeader>
            <CardContent>
              <div className="text-sm text-text-secondary space-y-1">
                <div><kbd className="px-1.5 py-0.5 rounded bg-surface-2 text-xs">Ctrl/Cmd + N</kbd> New chat</div>
                <div><kbd className="px-1.5 py-0.5 rounded bg-surface-2 text-xs">Enter</kbd> Send message</div>
                <div><kbd className="px-1.5 py-0.5 rounded bg-surface-2 text-xs">Shift + Enter</kbd> New line</div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="auth">
          <div className="flex flex-col gap-4">
            <AuthSettingsSection />
            <Card>
              <CardHeader><CardTitle className="text-text-primary text-base">Pending Approvals</CardTitle></CardHeader>
              <CardContent>
                {approvals.length === 0 && <div className="text-text-secondary text-sm">No pending approvals.</div>}
                {approvals.map((a) => (
                  <div key={a.id} className="bg-surface-2 border border-border-primary rounded-xl p-3 mt-2">
                    <div className="text-text-primary"><strong>{a.kind}</strong> — {a.summary}</div>
                    {a.command && <pre className="mt-1 text-xs bg-code-bg p-2 rounded-md overflow-x-auto">{a.command}</pre>}
                    {a.cwd && <div className="text-xs text-text-secondary">cwd: {a.cwd}</div>}
                    <div className="flex gap-2 mt-2">
                      <Button size="sm" variant="default" onClick={() => decide(a.id, true)}>Approve</Button>
                      <Button size="sm" variant="destructive" onClick={() => decide(a.id, false)}>Deny</Button>
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="app">
          <div className="flex flex-col gap-4">
            <AppSettingsSection />
            <Card>
              <CardHeader><CardTitle className="text-text-primary text-base">Diagnostics</CardTitle></CardHeader>
              <CardContent>
                <Button size="sm" variant="secondary" onClick={async () => {
                  try {
                    const r = await api.diagnostics();
                    const blob = new Blob([JSON.stringify(r, null, 2)], { type: "application/json" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url; a.download = "gaggle-diagnostics.json"; a.click();
                    URL.revokeObjectURL(url);
                  } catch (e) { setError(String(e)); }
                }}>Export Diagnostics</Button>
              </CardContent>
            </Card>
            <Card>
              <CardHeader><CardTitle className="text-text-primary text-base">Raw Settings</CardTitle></CardHeader>
              <CardContent>
                <table className="w-full text-sm">
                  <thead><tr className="border-b border-border-primary text-text-secondary"><th className="text-left py-2 px-3">Key</th><th className="text-left py-2 px-3">Value</th></tr></thead>
                  <tbody>
                    {settings.map((s) => (
                      <tr key={s.key} className="border-b border-border-primary">
                        <td className="py-2 px-3 text-text-primary">{s.key}</td>
                        <td className="py-2 px-3"><code className="text-xs">{s.value}</code></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}