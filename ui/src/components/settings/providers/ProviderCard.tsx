import React from "react";
import type * as T from "../../../types";
import { Card, CardContent } from "../../ui/Card";
import { Button } from "../../ui/Button";

export function ProviderCard({
  provider,
  onClick,
}: {
  provider: T.ProviderStatus;
  onClick?: () => void;
}) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-text-primary">{provider.provider.name}</div>
            <div className="text-xs text-text-tertiary">{provider.provider.kind}</div>
          </div>
          <Button size="sm" variant="secondary" onClick={onClick}>Configure</Button>
        </div>
        <div className="mt-2 text-xs text-text-secondary">
          {provider.has_configured_secret ? "Secret configured" : "No secret"}
        </div>
      </CardContent>
    </Card>
  );
}

export default ProviderCard;