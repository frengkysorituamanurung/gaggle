import React from "react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../ui/Tabs";
import { AppSettingsSection } from "./app/AppSettingsSection";
import { AuthSettingsSection } from "./auth/AuthSettingsSection";
import { ModelsSection } from "./models/ModelsSection";
import { AgentLoopSettings } from "./agent/AgentLoopSettings";
import { ExternalBackendSection } from "./agent/ExternalBackendSection";
import { KeyboardShortcutsSection } from "../KeyboardShortcutsSection";
import { ThemeSettingsSection } from "./ThemeSettingsSection";
import { LiveVoiceSettingsSection } from "../LiveVoiceSettingsSection";
import { DictationSettingsSection } from "../DictationSettingsSection";

export function SettingsView() {
  return (
    <div className="p-6 h-full overflow-y-auto">
      <h2 className="text-xl font-semibold text-text-primary mb-4">Settings</h2>
      <Tabs defaultValue="app">
        <TabsList className="mb-4 flex flex-wrap">
          <TabsTrigger value="app">App</TabsTrigger>
          <TabsTrigger value="models">Models</TabsTrigger>
          <TabsTrigger value="auth">Auth</TabsTrigger>
          <TabsTrigger value="agent">Agent</TabsTrigger>
          <TabsTrigger value="theme">Theme</TabsTrigger>
          <TabsTrigger value="voice">Voice</TabsTrigger>
          <TabsTrigger value="dictation">Dictation</TabsTrigger>
          <TabsTrigger value="shortcuts">Shortcuts</TabsTrigger>
        </TabsList>
        <TabsContent value="app"><AppSettingsSection /></TabsContent>
        <TabsContent value="models"><ModelsSection /></TabsContent>
        <TabsContent value="auth"><AuthSettingsSection /></TabsContent>
        <TabsContent value="agent">
          <AgentLoopSettings />
          <ExternalBackendSection />
        </TabsContent>
        <TabsContent value="theme"><ThemeSettingsSection /></TabsContent>
        <TabsContent value="voice"><LiveVoiceSettingsSection /></TabsContent>
        <TabsContent value="dictation"><DictationSettingsSection /></TabsContent>
        <TabsContent value="shortcuts"><KeyboardShortcutsSection /></TabsContent>
      </Tabs>
    </div>
  );
}

export default SettingsView;