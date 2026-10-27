import React, { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/Card";
import { Switch } from "../../ui/Switch";

export default function SecurityToggle() {
  const [shellApproval, setShellApproval] = useState(true);
  const [fsRestrict, setFsRestrict] = useState(true);

  useEffect(() => {
    api.settings.list().then((s) => {
      const m = Object.fromEntries(s.map((x) => [x.key, x.value]));
      if (m["security_shell_approval"] !== undefined) setShellApproval(m["security_shell_approval"] === "true");
      if (m["security_fs_restrict"] !== undefined) setFsRestrict(m["security_fs_restrict"] === "true");
    }).catch(() => {});
  }, []);

  const toggleShell = async () => {
    const next = !shellApproval;
    setShellApproval(next);
    try { await api.settings.upsert("security_shell_approval", next); } catch { /* ignore */ }
  };
  const toggleFs = async () => {
    const next = !fsRestrict;
    setFsRestrict(next);
    try { await api.settings.upsert("security_fs_restrict", next); } catch { /* ignore */ }
  };

  return (
    <Card>
      <CardHeader><CardTitle className="text-text-primary text-base">Security</CardTitle></CardHeader>
      <CardContent>
        <div className="flex items-center justify-between py-2">
          <span className="text-sm text-text-primary">Require approval for shell commands</span>
          <Switch checked={shellApproval} onCheckedChange={toggleShell} />
        </div>
        <div className="flex items-center justify-between py-2">
          <span className="text-sm text-text-primary">Restrict filesystem to project root</span>
          <Switch checked={fsRestrict} onCheckedChange={toggleFs} />
        </div>
      </CardContent>
    </Card>
  );
}
