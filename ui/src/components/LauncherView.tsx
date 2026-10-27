import React from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "./ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/Card";

export default function LauncherView() {
  const navigate = useNavigate();
  return (
    <div className="flex items-center justify-center h-full p-6">
      <Card className="w-full max-w-md">
        <CardHeader><CardTitle className="text-text-primary">Gaggle Launcher</CardTitle></CardHeader>
        <CardContent>
          <p className="text-text-secondary text-sm mb-3">Start a new chat or open settings.</p>
          <div className="flex gap-2">
            <Button variant="default" onClick={() => navigate("/")}>Home</Button>
            <Button variant="secondary" onClick={() => navigate("/settings")}>Settings</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}