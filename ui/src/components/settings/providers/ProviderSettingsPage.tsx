import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type * as T from "../../../types";
import { ProviderGrid } from "./ProviderGrid";
import { ProviderConfigurationModal } from "./modal/ProviderConfigurationModal";
import { Button } from "../../ui/Button";
import { EmptyState } from "../../EmptyState";
import { LoadingState } from "../../LoadingState";
import { ErrorState } from "../../ErrorState";

export function ProviderSettingsPage() {
  const [providers, setProviders] = useState<T.ProviderStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [selected, setSelected] = useState<T.ProviderStatus | null>(null);

  const refresh = () => {
    setLoading(true);
    api.providers.list().then(setProviders).catch((e) => setError(String(e))).finally(() => setLoading(false));
  };
  useEffect(refresh, []);

  if (loading) return <LoadingState />;
  if (error) return <ErrorState message={error} onRetry={refresh} />;
  if (providers.length === 0) return <EmptyState title="No providers" description="Add a provider to get started." />;

  return (
    <div className="p-6 h-full overflow-y-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-text-primary">Providers</h2>
        <Button size="sm" variant="default" onClick={() => { setSelected(null); setModalOpen(true); }}>Add Provider</Button>
      </div>
      <ProviderGrid providers={providers} onSelect={(p) => { setSelected(p); setModalOpen(true); }} />
      <ProviderConfigurationModal open={modalOpen} onClose={() => setModalOpen(false)} initial={selected} onSaved={refresh} />
    </div>
  );
}

export default ProviderSettingsPage;