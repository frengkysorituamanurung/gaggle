import React, { useState, useEffect } from "react";
import { api } from "../lib/api";
import type * as T from "../types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";

interface OnboardingScreenProps {
  onComplete: () => void;
}

export default function OnboardingScreen({ onComplete }: OnboardingScreenProps) {
  const [step, setStep] = useState(0);
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [providerName, setProviderName] = useState("");
  const [providerKind, setProviderKind] = useState("openai_compat");
  const [providerUrl, setProviderUrl] = useState("");
  const [providerKey, setProviderKey] = useState("");

  useEffect(() => {
    api.providers.list().then(setProviders).catch(() => {});
  }, []);

  const createProvider = async () => {
    try {
      await api.providers.create({
        kind: providerKind,
        name: providerName || providerKind,
        base_url: providerUrl || undefined,
        auth_kind: "api_key",
        api_key: providerKey || undefined,
      });
      const ps = await api.providers.list();
      setProviders(ps);
      setStep(2);
    } catch (e) {
      setError(String(e));
    }
  };

  const steps = [
    {
      title: "Welcome to Gaggle",
      content: (
        <div className="text-center">
          <p className="text-text-secondary mb-4">
            Gaggle is your local AI agent workspace. Let's get you set up.
          </p>
          <Button onClick={() => setStep(1)}>Get Started</Button>
        </div>
      ),
    },
    {
      title: "Configure a Provider",
      content: (
        <div className="flex flex-col gap-3">
          <p className="text-text-secondary text-sm">
            Add an AI provider to start chatting. You can add more later in Settings.
          </p>
          {error && <div className="text-text-danger text-sm">{error}</div>}
          <Input placeholder="Provider name" value={providerName} onChange={(e) => setProviderName(e.target.value)} />
          <select
            className="w-full px-3 py-2 rounded-md border border-border-primary bg-background-primary text-text-primary text-sm"
            value={providerKind}
            onChange={(e) => setProviderKind(e.target.value)}
          >
            <option value="openai">OpenAI</option>
            <option value="anthropic">Anthropic</option>
            <option value="google">Google Gemini</option>
            <option value="ollama">Ollama</option>
            <option value="openai_compat">OpenAI Compatible</option>
          </select>
          <Input placeholder="Base URL (optional)" value={providerUrl} onChange={(e) => setProviderUrl(e.target.value)} />
          <Input placeholder="API Key" value={providerKey} onChange={(e) => setProviderKey(e.target.value)} type="password" />
          <Button onClick={createProvider} disabled={!providerName.trim() && !providerKind}>
            Add Provider
          </Button>
          <Button variant="secondary" onClick={() => setStep(2)}>
            Skip for now
          </Button>
        </div>
      ),
    },
    {
      title: "You're Ready!",
      content: (
        <div className="text-center">
          <p className="text-text-secondary mb-4">
            {providers.length > 0
              ? `Provider configured: ${providers[0].provider.name}. You can start chatting now.`
              : "No provider configured yet. You can add one later in Settings."}
          </p>
          <Button onClick={onComplete}>Start Using Gaggle</Button>
        </div>
      ),
    },
  ];

  const current = steps[step];

  return (
    <div className="h-full flex items-center justify-center p-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle className="text-text-primary text-lg">{current.title}</CardTitle>
        </CardHeader>
        <CardContent>{current.content}</CardContent>
      </Card>
    </div>
  );
}